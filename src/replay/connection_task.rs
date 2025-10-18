use anyhow::Result;
use tokio::sync::mpsc;
use crate::profile::Event;
use super::client::ReplayClient;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Spawns a single connection task that processes commands from a queue
pub async fn spawn_connection_task(
    connection_id: u32,
    target: &str,
    rx: mpsc::Receiver<Event>,
    sent_counter: Arc<AtomicU64>,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let target = target.to_string();

    let handle = tokio::spawn(async move {
        let mut client = ReplayClient::new(&target).await?;
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
