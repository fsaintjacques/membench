use super::client::ReplayClient;
use super::stats::{ConnectionStats, StatsSnapshot};
use super::ProtocolMode;
use crate::profile::Event;
use anyhow::Result;
use std::time::Instant;
use tokio::sync::mpsc;

/// Spawns a single connection task that processes commands from a queue
pub async fn spawn_connection_task(
    target: &str,
    rx: mpsc::Receiver<Event>,
    stats_tx: mpsc::Sender<StatsSnapshot>,
    connection_id: u16,
    protocol_mode: ProtocolMode,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let target = target.to_string();

    let handle = tokio::spawn(async move {
        let mut client = ReplayClient::new(&target, protocol_mode).await?;
        let mut rx = rx;
        let mut local_stats = ConnectionStats::new(connection_id);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::debug!("Connection {} cancelled", connection_id);
                    break;
                }
                event_opt = rx.recv() => {
                    match event_opt {
                        Some(event) => {
                            let start = Instant::now();

                            if let Err(e) = client.send_command(&event).await {
                                local_stats.record_error(event.cmd_type, super::stats::ErrorType::ConnectionError);
                                return Err(e);
                            }

                            if let Err(e) = client.read_response().await {
                                local_stats.record_error(event.cmd_type, super::stats::ErrorType::ProtocolError);
                                return Err(e);
                            }

                            let latency = start.elapsed();
                            local_stats.record_success(event.cmd_type, latency);
                        }
                        None => {
                            // Channel closed
                            tracing::debug!("Connection {} channel closed", connection_id);
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    if stats_tx.send(local_stats.snapshot()).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            }
        }

        let _ = stats_tx.send(local_stats.snapshot()).await;
        tracing::debug!("Connection {} exiting", connection_id);
        Ok(())
    });

    Ok(handle)
}
