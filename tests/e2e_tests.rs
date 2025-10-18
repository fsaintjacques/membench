#[cfg(test)]
mod tests {
    use membench::{
        profile::{Event, CommandType, Flags},
        record::ProfileWriter,
        replay::{ProfileReader, DistributionAnalyzer, TrafficGenerator},
    };
    use tempfile::TempDir;

    #[test]
    fn test_capture_analyze_replay_workflow() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("test_profile.bin");
        let path = profile_path.to_str().unwrap();

        // Phase 1: Write a profile with various events
        println!("Phase 1: Creating profile...");
        let mut writer = ProfileWriter::new(path).unwrap();

        for i in 0..100 {
            let event = Event {
                timestamp: 1000 + i as u64,
                conn_id: i % 4,
                cmd_type: if i % 5 == 0 {
                    CommandType::Set
                } else {
                    CommandType::Get
                },
                key_hash: (i as u64).wrapping_mul(0x123456789),
                key_size: 10 + (i % 20) as u32,
                value_size: if i % 5 == 0 {
                    std::num::NonZero::new(100 + (i % 200) as u32)
                } else {
                    None
                },
                flags: Flags::empty(),
            };
            writer.write_event(&event).unwrap();
        }
        writer.finish().unwrap();

        // Phase 2: Read and verify profile
        println!("Phase 2: Reading profile...");
        let reader = ProfileReader::new(path).unwrap();
        let metadata = reader.metadata();

        assert_eq!(metadata.total_events, 100);
        assert_eq!(metadata.unique_connections, 4);
        println!("  Metadata verified: {} events, {} connections",
                 metadata.total_events, metadata.unique_connections);

        // Phase 3: Analyze distributions
        println!("Phase 3: Analyzing distributions...");
        let events = reader.events();
        let analysis = DistributionAnalyzer::analyze(events);

        assert_eq!(analysis.total_events, 100);
        assert!(analysis.command_distribution.contains_key(&CommandType::Get));
        assert!(analysis.command_distribution.contains_key(&CommandType::Set));

        let get_count = analysis.command_distribution.get(&CommandType::Get).unwrap_or(&0);
        let set_count = analysis.command_distribution.get(&CommandType::Set).unwrap_or(&0);
        println!("  Distribution: {} GET, {} SET",
                 get_count, set_count);

        // Phase 4: Generate traffic from analysis
        println!("Phase 4: Generating traffic from analysis...");
        let mut generator = TrafficGenerator::new(analysis);

        // Generate 10 commands and verify they're valid
        for _ in 0..10 {
            let cmd = generator.next_command();

            // Verify command is valid
            assert!(matches!(
                cmd.cmd_type,
                CommandType::Get | CommandType::Set | CommandType::Delete | CommandType::Noop
            ));

            // Verify Set commands have value_size
            if cmd.cmd_type == CommandType::Set {
                assert!(cmd.value_size.is_some());
            }

            // Verify Get commands don't have value_size
            if cmd.cmd_type == CommandType::Get {
                assert!(cmd.value_size.is_none());
            }
        }
        println!("  Generated 10 valid commands");

        println!("E2E test passed: Profile created, read, analyzed, and replayed successfully");
    }

    #[test]
    fn test_round_trip_serialization() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("roundtrip.bin");
        let path = profile_path.to_str().unwrap();

        // Create events with specific values
        let original_events = vec![
            Event {
                timestamp: 12345,
                conn_id: 7,
                cmd_type: CommandType::Get,
                key_hash: 0xdeadbeef,
                key_size: 42,
                value_size: None,
                flags: Flags::empty(),
            },
            Event {
                timestamp: 54321,
                conn_id: 3,
                cmd_type: CommandType::Set,
                key_hash: 0xcafebabe,
                key_size: 16,
                value_size: std::num::NonZero::new(256),
                flags: Flags::empty(),
            },
        ];

        // Write events
        let mut writer = ProfileWriter::new(path).unwrap();
        for event in &original_events {
            writer.write_event(event).unwrap();
        }
        writer.finish().unwrap();

        // Read events back
        let reader = ProfileReader::new(path).unwrap();
        let read_events = reader.events();

        // Verify they match
        assert_eq!(read_events.len(), original_events.len());
        for (original, read) in original_events.iter().zip(read_events.iter()) {
            assert_eq!(original.timestamp, read.timestamp);
            assert_eq!(original.conn_id, read.conn_id);
            assert_eq!(original.cmd_type, read.cmd_type);
            assert_eq!(original.key_hash, read.key_hash);
            assert_eq!(original.key_size, read.key_size);
            assert_eq!(original.value_size, read.value_size);
        }

        println!("Round-trip serialization test passed");
    }

    #[test]
    fn test_large_profile() {
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("large.bin");
        let path = profile_path.to_str().unwrap();

        // Create a large profile with 1000 events
        let event_count = 1000;
        let mut writer = ProfileWriter::new(path).unwrap();

        for i in 0..event_count {
            let event = Event {
                timestamp: (i as u64).wrapping_mul(1234567),
                conn_id: (i % 32) as u16,
                cmd_type: match i % 6 {
                    0 => CommandType::Get,
                    1 => CommandType::Set,
                    2 => CommandType::Delete,
                    _ => CommandType::Noop,
                },
                key_hash: (i as u64).wrapping_mul(987654321),
                key_size: (i % 256) as u32,
                value_size: if i % 2 == 0 {
                    std::num::NonZero::new((i as u32).wrapping_mul(256))
                } else {
                    None
                },
                flags: Flags::empty(),
            };
            writer.write_event(&event).unwrap();
        }
        writer.finish().unwrap();

        // Read it back
        let reader = ProfileReader::new(path).unwrap();
        let metadata = reader.metadata();
        let events = reader.events();

        assert_eq!(metadata.total_events, event_count as u64);
        assert_eq!(events.len(), event_count);

        // Analyze the large profile
        let analysis = DistributionAnalyzer::analyze(events);
        assert_eq!(analysis.total_events, event_count as u64);

        println!("Large profile test passed: {} events processed", event_count);
    }
}
