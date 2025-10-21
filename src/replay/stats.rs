use crate::profile::CommandType;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorType {
    Timeout,
    ConnectionError,
    ProtocolError,
}

#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub connection_id: u16,
    pub histograms: HashMap<CommandType, Histogram<u64>>,
    pub success_counts: HashMap<CommandType, u64>,
    pub error_counts: HashMap<ErrorType, u64>,
}

pub struct ConnectionStats {
    pub connection_id: u16,

    // Per-operation histograms (microsecond precision)
    histograms: HashMap<CommandType, Histogram<u64>>,

    // Success counters per operation
    success_counts: HashMap<CommandType, u64>,

    // Error tracking
    error_counts: HashMap<ErrorType, u64>,
}

impl ConnectionStats {
    pub fn new(connection_id: u16) -> Self {
        ConnectionStats {
            connection_id,
            histograms: HashMap::new(),
            success_counts: HashMap::new(),
            error_counts: HashMap::new(),
        }
    }

    pub fn record_success(&mut self, cmd_type: CommandType, latency: Duration) {
        let micros = latency.as_micros() as u64;

        // Update histogram
        let histogram = self.histograms
            .entry(cmd_type)
            .or_insert_with(|| Histogram::new(3).expect("Failed to create histogram"));
        histogram.record(micros).ok();

        // Update counter
        *self.success_counts.entry(cmd_type).or_insert(0) += 1;
    }

    pub fn record_error(&mut self, _cmd_type: CommandType, error_type: ErrorType) {
        *self.error_counts.entry(error_type).or_insert(0) += 1;
    }

    pub fn get_count(&self) -> u64 {
        self.success_counts.values().sum()
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_counts.values().sum()
    }

    /// Take a snapshot and reset counters (delta reporting)
    pub fn snapshot(&mut self) -> StatsSnapshot {
        let snapshot = StatsSnapshot {
            connection_id: self.connection_id,
            histograms: self.histograms.clone(),
            success_counts: self.success_counts.clone(),
            error_counts: self.error_counts.clone(),
        };

        // Reset for next interval
        self.histograms.clear();
        self.success_counts.clear();
        self.error_counts.clear();

        snapshot
    }
}

#[derive(Debug, Serialize)]
pub struct JsonStats {
    pub elapsed_secs: f64,
    pub total_operations: u64,
    pub throughput: f64,
    pub operations: HashMap<String, OperationStats>,
    pub errors: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct OperationStats {
    pub count: u64,
    pub p50_micros: u64,
    pub p95_micros: u64,
    pub p99_micros: u64,
    pub min_micros: u64,
    pub max_micros: u64,
}

pub struct AggregatedStats {
    // Merged histograms per operation type
    histograms: HashMap<CommandType, Histogram<u64>>,

    // Total counters
    success_counts: HashMap<CommandType, u64>,
    error_counts: HashMap<ErrorType, u64>,

    // Timing
    start_time: std::time::Instant,
}

impl Default for AggregatedStats {
    fn default() -> Self {
        Self::new()
    }
}

impl AggregatedStats {
    pub fn new() -> Self {
        AggregatedStats {
            histograms: HashMap::new(),
            success_counts: HashMap::new(),
            error_counts: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn merge(&mut self, snapshot: StatsSnapshot) {
        // Merge histograms
        for (cmd_type, hist) in snapshot.histograms {
            let agg_hist = self.histograms
                .entry(cmd_type)
                .or_insert_with(|| Histogram::new(3).expect("Failed to create histogram"));
            agg_hist.add(&hist).ok();
        }

        // Merge success counts
        for (cmd_type, count) in snapshot.success_counts {
            *self.success_counts.entry(cmd_type).or_insert(0) += count;
        }

        // Merge error counts
        for (error_type, count) in snapshot.error_counts {
            *self.error_counts.entry(error_type).or_insert(0) += count;
        }
    }

    pub fn total_operations(&self) -> u64 {
        self.success_counts.values().sum()
    }

    pub fn percentile(&self, cmd_type: CommandType, percentile: f64) -> Option<u64> {
        self.histograms
            .get(&cmd_type)
            .map(|h| h.value_at_percentile(percentile))
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn throughput(&self) -> f64 {
        let elapsed = self.elapsed_secs();
        if elapsed > 0.0 {
            self.total_operations() as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut operations = HashMap::new();

        for (cmd_type, hist) in &self.histograms {
            let count = self.success_counts.get(cmd_type).copied().unwrap_or(0);
            let op_stats = OperationStats {
                count,
                p50_micros: hist.value_at_percentile(50.0),
                p95_micros: hist.value_at_percentile(95.0),
                p99_micros: hist.value_at_percentile(99.0),
                min_micros: hist.min(),
                max_micros: hist.max(),
            };
            operations.insert(format!("{:?}", cmd_type), op_stats);
        }

        let mut errors = HashMap::new();
        for (error_type, count) in &self.error_counts {
            errors.insert(format!("{:?}", error_type), *count);
        }

        let json_stats = JsonStats {
            elapsed_secs: self.elapsed_secs(),
            total_operations: self.total_operations(),
            throughput: self.throughput(),
            operations,
            errors,
        };

        serde_json::to_string_pretty(&json_stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::CommandType;
    use std::time::Duration;

    #[test]
    fn test_connection_stats_creation() {
        let stats = ConnectionStats::new(1);
        assert_eq!(stats.connection_id, 1);
    }

    #[test]
    fn test_record_latency() {
        let mut stats = ConnectionStats::new(1);
        stats.record_success(CommandType::Get, Duration::from_micros(100));
        stats.record_success(CommandType::Get, Duration::from_micros(200));

        assert_eq!(stats.get_count(), 2);
    }

    #[test]
    fn test_record_error() {
        let mut stats = ConnectionStats::new(1);
        stats.record_error(CommandType::Set, ErrorType::Timeout);

        assert_eq!(stats.get_error_count(), 1);
    }

    #[test]
    fn test_snapshot_creation() {
        let mut stats = ConnectionStats::new(1);
        stats.record_success(CommandType::Get, Duration::from_micros(100));
        stats.record_success(CommandType::Set, Duration::from_micros(200));

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.connection_id, 1);
    }

    #[test]
    fn test_snapshot_reset() {
        let mut stats = ConnectionStats::new(1);
        stats.record_success(CommandType::Get, Duration::from_micros(100));

        let _snapshot = stats.snapshot();
        assert_eq!(stats.get_count(), 0); // Should be reset after snapshot
    }

    #[test]
    fn test_aggregated_stats_merge() {
        let mut agg = AggregatedStats::new();

        let mut stats1 = ConnectionStats::new(1);
        stats1.record_success(CommandType::Get, Duration::from_micros(100));

        let mut stats2 = ConnectionStats::new(2);
        stats2.record_success(CommandType::Get, Duration::from_micros(200));

        agg.merge(stats1.snapshot());
        agg.merge(stats2.snapshot());

        assert_eq!(agg.total_operations(), 2);
    }

    #[test]
    fn test_aggregated_percentiles() {
        let mut agg = AggregatedStats::new();

        let mut stats = ConnectionStats::new(1);
        for i in 1..=100 {
            stats.record_success(CommandType::Get, Duration::from_micros(i * 10));
        }

        agg.merge(stats.snapshot());

        let p50 = agg.percentile(CommandType::Get, 50.0);
        assert!(p50.is_some());
    }

    #[test]
    fn test_json_export() {
        let mut agg = AggregatedStats::new();

        let mut stats = ConnectionStats::new(1);
        stats.record_success(CommandType::Get, Duration::from_micros(100));
        stats.record_success(CommandType::Set, Duration::from_micros(200));

        agg.merge(stats.snapshot());

        let json = agg.to_json().expect("Failed to serialize");
        assert!(json.contains("\"Get\""));
        assert!(json.contains("\"Set\""));
    }
}
