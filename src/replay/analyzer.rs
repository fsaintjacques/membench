use std::collections::HashMap;
use crate::profile::{Event, CommandType};

#[derive(Clone)]
pub struct AnalysisResult {
    pub total_events: u64,
    pub command_distribution: HashMap<CommandType, u64>,
    pub key_size_distribution: Vec<(u32, u64)>,
    pub value_size_distribution: Vec<(u32, u64)>,
}

pub struct DistributionAnalyzer;

impl DistributionAnalyzer {
    pub fn analyze(events: &[Event]) -> AnalysisResult {
        let mut cmd_dist = HashMap::new();
        let mut key_size_dist = HashMap::new();
        let mut value_size_dist = HashMap::new();

        for event in events {
            *cmd_dist.entry(event.cmd_type).or_insert(0) += 1;
            *key_size_dist.entry(event.key_size).or_insert(0) += 1;

            if let Some(size) = event.value_size {
                *value_size_dist.entry(size.get()).or_insert(0) += 1;
            }
        }

        AnalysisResult {
            total_events: events.len() as u64,
            command_distribution: cmd_dist,
            key_size_distribution: key_size_dist
                .into_iter()
                .collect::<Vec<_>>(),
            value_size_distribution: value_size_dist
                .into_iter()
                .collect::<Vec<_>>(),
        }
    }
}
