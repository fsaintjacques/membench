#[cfg(test)]
mod tests {
    use membench::replay::DistributionAnalyzer;
    use membench::profile::{Event, CommandType, Flags};

    #[test]
    fn test_analyze_command_distribution() {
        let events = vec![
            Event {
                timestamp: 1000,
                conn_id: 1,
                cmd_type: CommandType::Get,
                key_hash: 0x1,
                key_size: 10,
                value_size: None,
                flags: Flags::empty(),
            },
            Event {
                timestamp: 2000,
                conn_id: 1,
                cmd_type: CommandType::Set,
                key_hash: 0x2,
                key_size: 20,
                value_size: std::num::NonZero::new(50),
                flags: Flags::empty(),
            },
        ];

        let analysis = DistributionAnalyzer::analyze(&events);

        assert_eq!(analysis.total_events, 2);
        assert_eq!(analysis.command_distribution.get(&CommandType::Get), Some(&1));
        assert_eq!(analysis.command_distribution.get(&CommandType::Set), Some(&1));
    }

    #[test]
    fn test_analyze_hit_rate() {
        let events = vec![
            Event {
                timestamp: 1000,
                conn_id: 1,
                cmd_type: CommandType::Get,
                key_hash: 0x1,
                key_size: 10,
                value_size: None,
                flags: Flags::empty(),
            },
            Event {
                timestamp: 2000,
                conn_id: 1,
                cmd_type: CommandType::Get,
                key_hash: 0x2,
                key_size: 10,
                value_size: None,
                flags: Flags::empty(),
            },
        ];

        let analysis = DistributionAnalyzer::analyze(&events);

        // Test that we have the correct number of events
        assert_eq!(analysis.total_events, 2);
        assert_eq!(analysis.command_distribution.get(&CommandType::Get), Some(&2));
    }

    #[test]
    fn test_analyze_size_distributions() {
        let events = vec![
            Event {
                timestamp: 1000,
                conn_id: 1,
                cmd_type: CommandType::Get,
                key_hash: 0x1,
                key_size: 10,
                value_size: None,
                flags: Flags::empty(),
            },
            Event {
                timestamp: 2000,
                conn_id: 1,
                cmd_type: CommandType::Set,
                key_hash: 0x2,
                key_size: 20,
                value_size: std::num::NonZero::new(50),
                flags: Flags::empty(),
            },
        ];

        let analysis = DistributionAnalyzer::analyze(&events);

        // Key size distribution should have both 10 and 20
        assert!(analysis.key_size_distribution.iter().any(|(size, count)| *size == 10 && *count == 1));
        assert!(analysis.key_size_distribution.iter().any(|(size, count)| *size == 20 && *count == 1));

        // Value size distribution should have 50
        assert!(analysis.value_size_distribution.iter().any(|(size, count)| *size == 50 && *count == 1));
    }
}
