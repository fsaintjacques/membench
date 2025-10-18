use anyhow::Result;
use tokio::sync::mpsc;
use std::collections::HashMap;
use crate::profile::Event;

pub enum LoopMode {
    Once,
    Times(usize),
    Infinite,
}

/// Main reader task: streams events from profile, routes to connection queues, handles looping
pub async fn reader_task(
    profile_path: &str,
    connection_queues: HashMap<u32, mpsc::Sender<Event>>,
    loop_mode: LoopMode,
    should_exit: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<()> {
    use super::streamer::ProfileStreamer;
    use std::sync::atomic::Ordering;

    let mut streamer = ProfileStreamer::new(profile_path)?;

    let loop_count = match loop_mode {
        LoopMode::Once => 1,
        LoopMode::Times(n) => n,
        LoopMode::Infinite => usize::MAX,
    };

    for iteration in 0..loop_count {
        if should_exit.load(Ordering::Relaxed) {
            tracing::info!("Reader task exiting due to signal");
            break;
        }

        tracing::debug!("Reader task iteration {}", iteration);

        loop {
            match streamer.next()? {
                Some(event) => {
                    let conn_id = event.conn_id;

                    if let Some(tx) = connection_queues.get(&conn_id) {
                        if tx.send(event).await.is_err() {
                            tracing::warn!("Connection {} task closed unexpectedly", conn_id);
                            break;
                        }
                    } else {
                        tracing::warn!("Unknown connection ID: {}", conn_id);
                    }

                    if should_exit.load(Ordering::Relaxed) {
                        tracing::info!("Reader task exiting due to signal");
                        return Ok(());
                    }
                }
                None => {
                    // End of profile file
                    if iteration < loop_count - 1 {
                        tracing::debug!("End of profile, resetting for next iteration");
                        streamer.reset()?;
                    } else {
                        tracing::info!("All replay iterations complete");
                        return Ok(());
                    }
                    break;
                }
            }
        }
    }

    // Close all queues to signal connection tasks to exit
    drop(connection_queues);

    Ok(())
}
