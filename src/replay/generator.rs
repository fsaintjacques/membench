use crate::profile::{CommandType, Event, Flags};
use rand::Rng;
use super::analyzer::AnalysisResult;

pub struct TrafficGenerator {
    analysis: AnalysisResult,
    rng: rand::rngs::ThreadRng,
}

impl TrafficGenerator {
    pub fn new(analysis: AnalysisResult) -> Self {
        TrafficGenerator {
            analysis,
            rng: rand::thread_rng(),
        }
    }

    pub fn next_command(&mut self) -> Event {
        let cmd_type = self.sample_command();
        let key_size = self.sample_key_size();
        let value_size = if cmd_type == CommandType::Set {
            std::num::NonZero::new(self.sample_value_size())
        } else {
            None
        };

        Event {
            timestamp: self.rng.gen::<u64>(),
            conn_id: self.rng.gen::<u16>() % 4, // Limit to 4 connections
            cmd_type,
            key_hash: self.rng.gen::<u64>(),
            key_size,
            value_size,
            flags: Flags::empty(),
        }
    }

    fn sample_command(&mut self) -> CommandType {
        let total: u64 = self.analysis.command_distribution.values().sum();
        if total == 0 {
            return CommandType::Get;
        }

        let mut r = self.rng.gen::<u64>() % total;

        for (cmd, count) in &self.analysis.command_distribution {
            if r < *count {
                return *cmd;
            }
            r -= count;
        }

        CommandType::Get
    }

    fn sample_key_size(&mut self) -> u32 {
        if self.analysis.key_size_distribution.is_empty() {
            return 10;
        }

        let total: u64 = self.analysis.key_size_distribution.iter().map(|(_, c)| c).sum();
        if total == 0 {
            return 10;
        }

        let mut r = self.rng.gen::<u64>() % total;

        for (size, count) in &self.analysis.key_size_distribution {
            if r < *count {
                return *size;
            }
            r -= count;
        }

        10
    }

    fn sample_value_size(&mut self) -> u32 {
        if self.analysis.value_size_distribution.is_empty() {
            return 100;
        }

        let total: u64 = self.analysis.value_size_distribution.iter().map(|(_, c)| c).sum();
        if total == 0 {
            return 100;
        }

        let mut r = self.rng.gen::<u64>() % total;

        for (size, count) in &self.analysis.value_size_distribution {
            if r < *count {
                return *size;
            }
            r -= count;
        }

        100
    }
}
