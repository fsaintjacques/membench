use super::stats::{AggregatedStats, StatsSnapshot};
use tokio::sync::mpsc;

pub async fn spawn_stats_aggregator(
    mut rx: mpsc::Receiver<StatsSnapshot>,
    cancel_token: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<AggregatedStats> {
    tokio::spawn(async move {
        let mut agg_stats = AggregatedStats::new();
        let mut report_interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!("Stats aggregator cancelled");
                    break;
                }
                snapshot_opt = rx.recv() => {
                    match snapshot_opt {
                        Some(snapshot) => {
                            agg_stats.merge(snapshot);
                        }
                        None => {
                            tracing::info!("Stats aggregator receiver closed");
                            break;
                        }
                    }
                }
                _ = report_interval.tick() => {
                    // Live progress report
                    let elapsed = agg_stats.elapsed_secs();
                    let total_ops = agg_stats.total_operations();
                    let throughput = agg_stats.throughput();

                    // Skip the first report if interval not reached
                    if elapsed < report_interval.period().as_secs_f64() {
                        continue;
                    }

                    tracing::info!(
                        "[{:.0}s] Operations: {} | Throughput: {:.0} ops/sec",
                        elapsed,
                        total_ops,
                        throughput
                    );
                }
            }
        }

        agg_stats
    })
}
