use super::stats::{AggregatedStats, StatsSnapshot};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn spawn_stats_aggregator(
    mut rx: mpsc::Receiver<StatsSnapshot>,
    should_exit: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<AggregatedStats> {
    tokio::spawn(async move {
        let mut agg_stats = AggregatedStats::new();
        let mut report_interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(snapshot) = rx.recv() => {
                    agg_stats.merge(snapshot);
                }
                _ = report_interval.tick() => {
                    // Live progress report
                    let elapsed = agg_stats.elapsed_secs();
                    let total_ops = agg_stats.total_operations();
                    let throughput = agg_stats.throughput();

                    tracing::info!(
                        "[{:.0}s] Operations: {} | Throughput: {:.0} ops/sec",
                        elapsed,
                        total_ops,
                        throughput
                    );
                }
                else => {
                    if should_exit.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }

        agg_stats
    })
}
