use crate::profile::CommandType;
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Timeout,
    ConnectionError,
    ProtocolError,
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
}
