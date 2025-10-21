//! Replay command: stream profile events to memcache server with connection topology preservation

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::profile::{CommandType, Event};
use crate::replay::{
    reader_task, spawn_connection_task, spawn_stats_aggregator, stats::StatsSnapshot, LoopMode,
    ProfileReader, ProtocolMode,
};

pub async fn run(
    input: &str,
    target: &str,
    loop_mode: &str,
    protocol_mode: ProtocolMode,
    should_exit: Arc<AtomicBool>,
    stats_json: Option<&str>,
) -> Result<()> {
    tracing::info!(
        "Starting replay: input={}, target={}, mode={}, protocol={}",
        input,
        target,
        loop_mode,
        protocol_mode
    );

    // Parse loop mode
    let loop_mode = match loop_mode {
        "once" => LoopMode::Once,
        "infinite" => LoopMode::Infinite,
        s if s.starts_with("times:") => {
            let count = s
                .strip_prefix("times:")
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

    // Phase 1.5: Create stats aggregator
    let (stats_tx, stats_rx) = mpsc::channel::<StatsSnapshot>(1000);
    let stats_handle = spawn_stats_aggregator(stats_rx, Arc::clone(&should_exit)).await;

    // Phase 2: Create SPSC queues for each connection
    let mut connection_queues: HashMap<u16, mpsc::Sender<Event>> = HashMap::new();
    let mut connection_tasks = Vec::new();

    for &conn_id in &unique_connections {
        let (tx, rx) = mpsc::channel(1000); // Buffer size: 1000 events
        connection_queues.insert(conn_id, tx);

        let target = target.to_string();
        let stats_tx_clone = stats_tx.clone();
        let should_exit_clone = Arc::clone(&should_exit);

        let task_handle = spawn_connection_task(
            &target,
            rx,
            stats_tx_clone,
            conn_id,
            protocol_mode,
            should_exit_clone,
        )
        .await?;
        connection_tasks.push(task_handle);
    }

    // Drop our copy of stats_tx so aggregator can finish
    drop(stats_tx);

    // Phase 3: Spawn reader task
    let reader_task_handle = {
        let input_clone = input.to_string();
        let should_exit_clone = Arc::clone(&should_exit);

        tokio::spawn(async move {
            reader_task(
                &input_clone,
                connection_queues,
                loop_mode,
                should_exit_clone,
            )
            .await
        })
    };

    // Phase 4: Wait for reader task to complete (signals that all events processed)
    reader_task_handle.await??;

    // Phase 5: Wait for all connection tasks to drain queues and finish
    for task in connection_tasks {
        task.await??;
    }

    // Phase 6: Signal stats aggregator and get final results
    should_exit.store(true, Ordering::Relaxed);
    let final_stats = stats_handle.await?;

    // Final summary
    print_final_summary(&final_stats);

    // Export JSON if requested
    if let Some(json_path) = stats_json {
        let json = final_stats.to_json()?;
        std::fs::write(json_path, json)?;
        tracing::info!("Statistics exported to {}", json_path);
    }

    Ok(())
}

fn print_final_summary(stats: &crate::replay::stats::AggregatedStats) {
    tracing::info!("=== Replay Complete ===");
    tracing::info!("Elapsed: {:.2}s", stats.elapsed_secs());
    tracing::info!("Total Operations: {}", stats.total_operations());
    tracing::info!("Throughput: {:.2} ops/sec", stats.throughput());

    for cmd_type in [
        CommandType::Get,
        CommandType::Set,
        CommandType::Delete,
        CommandType::Noop,
    ] {
        if let Some(p50) = stats.percentile(cmd_type, 50.0) {
            let p95 = stats.percentile(cmd_type, 95.0).unwrap_or(0);
            let p99 = stats.percentile(cmd_type, 99.0).unwrap_or(0);

            tracing::info!(
                "{:?} latency (μs) - p50: {}, p95: {}, p99: {}",
                cmd_type,
                p50,
                p95,
                p99
            );
        }
    }
}
