//! Replay command: stream profile events to memcache server with connection topology preservation

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Instant;

use crate::replay::{
    ProfileReader,
    spawn_connection_task,
    reader_task,
    LoopMode,
};
use crate::profile::Event;

pub async fn run(input: &str, target: &str, loop_mode: &str, should_exit: Arc<AtomicBool>) -> Result<()> {
    tracing::info!("Starting replay: input={}, target={}, mode={}", input, target, loop_mode);

    // Parse loop mode
    let loop_mode = match loop_mode {
        "once" => LoopMode::Once,
        "infinite" => LoopMode::Infinite,
        s if s.starts_with("times:") => {
            let count = s.strip_prefix("times:")
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| anyhow::anyhow!("Invalid loop mode: {}", s))?;
            LoopMode::Times(count)
        }
        _ => LoopMode::Once,
    };

    // Phase 1: Read profile metadata and identify unique connections
    let reader = ProfileReader::new(input)?;
    let mut unique_connections = HashSet::<u16>::new();
    for event in reader.events() {
        unique_connections.insert(event.conn_id);
    }
    let unique_connections: Vec<u16> = unique_connections.into_iter().collect();
    tracing::info!("Found {} unique connections", unique_connections.len());

    // Phase 2: Create SPSC queues for each connection
    let mut connection_queues: HashMap<u16, mpsc::Sender<Event>> = HashMap::new();
    let mut connection_tasks = Vec::new();
    let sent_counter = Arc::new(AtomicU64::new(0));

    for &conn_id in &unique_connections {
        let (tx, rx) = mpsc::channel(1000); // Buffer size: 1000 events
        connection_queues.insert(conn_id, tx);

        let counter_clone = Arc::clone(&sent_counter);
        let target = target.to_string();

        let task_handle = spawn_connection_task(conn_id, &target, rx, counter_clone).await?;
        connection_tasks.push(task_handle);
    }

    // Phase 3: Spawn reader task
    let reader_task_handle = {
        let input_clone = input.to_string();
        let should_exit_clone = Arc::clone(&should_exit);

        tokio::spawn(async move {
            reader_task(&input_clone, connection_queues, loop_mode, should_exit_clone).await
        })
    };

    // Phase 4: Spawn reporting task
    let _reporting_handle = {
        let counter_clone = Arc::clone(&sent_counter);
        let should_exit_clone = Arc::clone(&should_exit);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            let start = Instant::now();

            loop {
                interval.tick().await;

                let sent = counter_clone.load(Ordering::Relaxed);
                let elapsed = start.elapsed().as_secs_f64();
                let throughput = if elapsed > 0.0 {
                    sent as f64 / elapsed
                } else {
                    0.0
                };

                tracing::info!(
                    "[{:.0}s] Sent: {} | Throughput: {:.0} ops/sec",
                    elapsed,
                    sent,
                    throughput
                );

                if should_exit_clone.load(Ordering::Relaxed) {
                    break;
                }
            }
        })
    };

    // Phase 5: Wait for reader task to complete (signals that all events processed)
    reader_task_handle.await??;

    // Phase 6: Wait for all connection tasks to drain queues and finish
    for task in connection_tasks {
        task.await??;
    }

    // Final stats
    let final_sent = sent_counter.load(Ordering::Relaxed);
    tracing::info!("Replay complete. Total sent: {}", final_sent);

    Ok(())
}
