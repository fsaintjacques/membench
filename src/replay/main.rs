//! Replay command: stream profile events to memcache server with connection topology preservation

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

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

    // Create cancellation token for coordinated shutdown
    let cancel_token = CancellationToken::new();

    // Spawn signal handler to trigger cancellation on Ctrl+C
    let cancel_token_for_signal = cancel_token.clone();
    tokio::spawn(async move {
        loop {
            if should_exit.load(Ordering::Relaxed) {
                tracing::info!("External exit signal received, cancelling all tasks");
                cancel_token_for_signal.cancel();
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

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
    let stats_handle = spawn_stats_aggregator(stats_rx, cancel_token.clone()).await;

    // Phase 2: Create SPSC queues for each connection
    let mut connection_queues: HashMap<u16, mpsc::Sender<Event>> = HashMap::new();
    let mut connection_tasks = Vec::new();

    for &conn_id in &unique_connections {
        let (tx, rx) = mpsc::channel(1000); // Buffer size: 1000 events
        connection_queues.insert(conn_id, tx);

        let target = target.to_string();
        let stats_tx_clone = stats_tx.clone();

        let task_handle = spawn_connection_task(
            &target,
            rx,
            stats_tx_clone,
            conn_id,
            protocol_mode,
            cancel_token.clone(),
        )
        .await?;
        connection_tasks.push(task_handle);
    }

    // Drop our copy of stats_tx so aggregator can finish when all connections close
    drop(stats_tx);

    // Phase 3: Spawn reader task
    let reader_task_handle = {
        let input_clone = input.to_string();
        let cancel_token_clone = cancel_token.clone();

        tokio::spawn(async move {
            reader_task(
                &input_clone,
                connection_queues,
                loop_mode,
                cancel_token_clone,
            )
            .await
        })
    };

    // Phase 4: Wait for reader task to complete (signals that all events processed)
    reader_task_handle.await??;
    tracing::info!("Reader task completed");

    // Phase 5: Wait for all connection tasks to drain queues and finish
    for (idx, task) in connection_tasks.into_iter().enumerate() {
        task.await??;
        tracing::debug!("Connection task {} completed", idx);
    }
    tracing::info!("All connection tasks completed");

    // Phase 6: Cancel stats aggregator and get final results
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
