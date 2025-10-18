#[cfg(test)]
mod tests {
    use membench::replay::{TrafficGenerator, AnalysisResult};
    use membench::profile::CommandType;
    use std::collections::HashMap;

    #[test]
    fn test_generator_produces_commands() {
        let mut cmd_dist = HashMap::new();
        cmd_dist.insert(CommandType::Get, 80);
        cmd_dist.insert(CommandType::Set, 20);

        let analysis = AnalysisResult {
            total_events: 100,
            command_distribution: cmd_dist,
            key_size_distribution: vec![(10, 50), (20, 50)],
            value_size_distribution: vec![(100, 50), (200, 50)],
        };

        let mut gen = TrafficGenerator::new(analysis);
        let cmd = gen.next_command();

        assert!(matches!(
            cmd.cmd_type,
            CommandType::Get | CommandType::Set
        ));
    }

    #[test]
    fn test_generator_respects_command_distribution() {
        let mut cmd_dist = HashMap::new();
        cmd_dist.insert(CommandType::Get, 100);
        cmd_dist.insert(CommandType::Set, 0); // No sets

        let analysis = AnalysisResult {
            total_events: 100,
            command_distribution: cmd_dist,
            key_size_distribution: vec![(10, 100)],
            value_size_distribution: vec![(100, 100)],
        };

        let mut gen = TrafficGenerator::new(analysis);
        for _ in 0..10 {
            let cmd = gen.next_command();
            assert_eq!(cmd.cmd_type, CommandType::Get);
        }
    }

    #[test]
    fn test_generator_set_has_value_size() {
        let mut cmd_dist = HashMap::new();
        cmd_dist.insert(CommandType::Set, 100);

        let analysis = AnalysisResult {
            total_events: 100,
            command_distribution: cmd_dist,
            key_size_distribution: vec![(10, 100)],
            value_size_distribution: vec![(100, 100)],
        };

        let mut gen = TrafficGenerator::new(analysis);
        let cmd = gen.next_command();

        assert_eq!(cmd.cmd_type, CommandType::Set);
        assert!(cmd.value_size.is_some());
    }

    #[test]
    fn test_generator_get_has_no_value_size() {
        let mut cmd_dist = HashMap::new();
        cmd_dist.insert(CommandType::Get, 100);

        let analysis = AnalysisResult {
            total_events: 100,
            command_distribution: cmd_dist,
            key_size_distribution: vec![(10, 100)],
            value_size_distribution: vec![],
        };

        let mut gen = TrafficGenerator::new(analysis);
        let cmd = gen.next_command();

        assert_eq!(cmd.cmd_type, CommandType::Get);
        assert!(cmd.value_size.is_none());
    }
}
