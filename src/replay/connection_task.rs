use super::client::ReplayClient;
use super::ProtocolMode;
use crate::profile::Event;
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Spawns a single connection task that processes commands from a queue
pub async fn spawn_connection_task(
    target: &str,
    rx: mpsc::Receiver<Event>,
    sent_counter: Arc<AtomicU64>,
    protocol_mode: ProtocolMode,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let target = target.to_string();

    let handle = tokio::spawn(async move {
        let mut client = ReplayClient::new(&target, protocol_mode).await?;
        let mut rx = rx;

        while let Some(event) = rx.recv().await {
            client.send_command(&event).await?;
            client.read_response().await?;
            sent_counter.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    });

    Ok(handle)
}
