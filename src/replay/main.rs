//! Replay command implementation

use anyhow::Result;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::replay::{ProfileReader, DistributionAnalyzer, ReplayClient};

pub async fn run(input: &str, target: &str, concurrency: usize) -> Result<()> {
    let reader = ProfileReader::new(input)?;
    let events = reader.events().to_vec();
    let analysis = DistributionAnalyzer::analyze(&events);

    println!("\n╔════════════════════════════════════════════╗");
    println!("║         Replay Statistics                  ║");
    println!("╚════════════════════════════════════════════╝\n");
    println!("Profile: {}", input);
    println!("Target: {}", target);
    println!("Concurrency: {}", concurrency);
    println!("Total events in profile: {}", analysis.total_events);
    println!("Command distribution:");
    for (cmd, count) in &analysis.command_distribution {
        println!("  {:?}: {}", cmd, count);
    }
    println!("Cache hit rate: {:.2}%\n", analysis.hit_rate * 100.0);
    println!("Starting replay... (Press Ctrl+C to stop)\n");

    // Create connection pool - reuse connections instead of creating new ones
    let mut pool = Vec::with_capacity(concurrency);
    for _ in 0..concurrency {
        match ReplayClient::new(target, 65536) {
            Ok(client) => pool.push(client),
            Err(e) => {
                tracing::warn!("Failed to create connection: {}", e);
                return Err(e);
            }
        }
    }

    let sent = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();

    let sent_clone = Arc::clone(&sent);
    let errors_clone = Arc::clone(&errors);

    // Spawn reporting task
    let _reporting_handle = tokio::spawn(async move {
        let mut last_report = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            if last_report.elapsed() >= Duration::from_secs(5) {
                let elapsed = start_time.elapsed().as_secs_f64();
                let sent_count = sent_clone.load(Ordering::Relaxed);
                let error_count = errors_clone.load(Ordering::Relaxed);
                let throughput = sent_count as f64 / elapsed;
                let error_rate = if sent_count > 0 {
                    (error_count as f64 / (sent_count + error_count) as f64) * 100.0
                } else {
                    0.0
                };

                println!(
                    "[{:6}s] Sent: {:8} | Errors: {:6} | Throughput: {:8.0} ops/sec | Error rate: {:.2}%",
                    elapsed as u64, sent_count, error_count, throughput, error_rate
                );
                last_report = Instant::now();
            }
        }
    });

    // Main replay loop - replay actual events from profile
    let mut event_index = 0;
    let total_events = events.len();
    let mut pool_index = 0;

    loop {
        for _ in 0..concurrency {
            // Stop when we've replayed all events
            if event_index >= total_events {
                let elapsed = start_time.elapsed().as_secs_f64();
                let sent_count = sent.load(Ordering::Relaxed);
                let error_count = errors.load(Ordering::Relaxed);
                let throughput = sent_count as f64 / elapsed;
                let error_rate = if sent_count > 0 {
                    (error_count as f64 / (sent_count + error_count) as f64) * 100.0
                } else {
                    0.0
                };

                println!(
                    "\n[{:6}s] Sent: {:8} | Errors: {:6} | Throughput: {:8.0} ops/sec | Error rate: {:.2}%",
                    elapsed as u64, sent_count, error_count, throughput, error_rate
                );
                println!("\n✓ Replay complete - all {} events sent\n", event_index);
                _reporting_handle.abort();
                return Ok(());
            }

            let event = &events[event_index];
            let client = &mut pool[pool_index % concurrency];
            pool_index += 1;
            event_index += 1;

            match client.send_command(event) {
                Ok(_) => {
                    // Read response to drain the socket buffer and prevent deadlock
                    match client.read_response() {
                        Ok(_) => {
                            sent.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            errors.fetch_add(1, Ordering::Relaxed);
                            tracing::warn!("Read error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!("Send error: {}", e);
                }
            }
        }
    }
}
