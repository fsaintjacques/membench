use membench::profile::CommandType;
use membench::replay::stats::{AggregatedStats, ConnectionStats};
use std::time::Duration;

#[test]
fn test_stats_workflow() {
    // Simulate connection task workflow
    let mut conn1 = ConnectionStats::new(1);
    let mut conn2 = ConnectionStats::new(2);

    // Simulate events
    for i in 1..=50 {
        conn1.record_success(CommandType::Get, Duration::from_micros(i * 10));
        conn2.record_success(CommandType::Set, Duration::from_micros(i * 20));
    }

    // Take snapshots
    let snap1 = conn1.snapshot();
    let snap2 = conn2.snapshot();

    // Aggregate
    let mut agg = AggregatedStats::new();
    agg.merge(snap1);
    agg.merge(snap2);

    // Verify
    assert_eq!(agg.total_operations(), 100);
    assert!(agg.percentile(CommandType::Get, 50.0).is_some());
    assert!(agg.percentile(CommandType::Set, 50.0).is_some());

    // JSON export
    let json = agg.to_json().expect("Failed to export JSON");
    assert!(json.contains("Get"));
    assert!(json.contains("Set"));
}

#[test]
fn test_stats_reset_after_snapshot() {
    let mut stats = ConnectionStats::new(1);
    stats.record_success(CommandType::Get, Duration::from_micros(100));

    let snapshot = stats.snapshot();
    assert_eq!(snapshot.success_counts.get(&CommandType::Get), Some(&1));

    // After snapshot, stats should be reset
    assert_eq!(stats.get_count(), 0);
}
