use membench::replay::stats::ConnectionStats;
use membench::profile::CommandType;
use std::time::Duration;

#[test]
fn test_connection_stats_tracking() {
    let mut stats = ConnectionStats::new(1);

    // Simulate tracking a request
    let start = std::time::Instant::now();
    std::thread::sleep(Duration::from_micros(100));
    let latency = start.elapsed();

    stats.record_success(CommandType::Get, latency);

    assert_eq!(stats.get_count(), 1);
    assert!(stats.get_count() > 0);
}
