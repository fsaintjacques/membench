use crate::profile::Event;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub enum LoopMode {
    Once,
    Times(usize),
    Infinite,
}

/// Main reader task: streams events from profile, routes to connection queues, handles looping
pub async fn reader_task(
    profile_path: &str,
    connection_queues: HashMap<u16, mpsc::Sender<Event>>,
    loop_mode: LoopMode,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<()> {
    use super::streamer::ProfileStreamer;

    let mut streamer = ProfileStreamer::new(profile_path)?;

    let loop_count = match loop_mode {
        LoopMode::Once => 1,
        LoopMode::Times(n) => n,
        LoopMode::Infinite => usize::MAX,
    };

    // Ensure queues are always closed on exit using a guard
    struct QueueGuard(Option<HashMap<u16, mpsc::Sender<Event>>>);
    impl Drop for QueueGuard {
        fn drop(&mut self) {
            if let Some(queues) = self.0.take() {
                tracing::debug!("Closing {} connection queues", queues.len());
                drop(queues);
            }
        }
    }
    let guard = QueueGuard(Some(connection_queues));
    let connection_queues = guard.0.as_ref().unwrap();

    for iteration in 0..loop_count {
        if cancel_token.is_cancelled() {
            tracing::info!("Reader task cancelled");
            break;
        }

        tracing::debug!("Reader task iteration {}", iteration);

        loop {
            // Check cancellation before processing next event
            if cancel_token.is_cancelled() {
                tracing::info!("Reader task cancelled during event processing");
                break;
            }

            // Read next event synchronously
            match streamer.next_event()? {
                Some(event) => {
                    let conn_id = event.conn_id;

                    if let Some(tx) = connection_queues.get(&conn_id) {
                        // Send event to connection queue with cancellation awareness
                        tokio::select! {
                            _ = cancel_token.cancelled() => {
                                tracing::info!("Reader task cancelled during send");
                                break;
                            }
                            result = tx.send(event) => {
                                if result.is_err() {
                                    tracing::warn!("Connection {} task closed unexpectedly", conn_id);
                                    break;
                                }
                            }
                        }
                    } else {
                        tracing::warn!("Unknown connection ID: {}", conn_id);
                    }
                }
                None => {
                    // End of profile file
                    if iteration < loop_count - 1 {
                        tracing::debug!("End of profile, resetting for next iteration");
                        streamer.reset()?;
                    } else {
                        tracing::info!("All replay iterations complete");
                    }
                    break;
                }
            }
        }
    }

    // Guard will automatically drop queues when function exits
    Ok(())
}
