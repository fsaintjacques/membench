use std::collections::HashMap;
use crate::profile::{Event, CommandType, Response};

pub struct AnalysisResult {
    pub total_events: u64,
    pub command_distribution: HashMap<CommandType, u64>,
    pub key_size_distribution: Vec<(u32, u64)>,
    pub value_size_distribution: Vec<(u32, u64)>,
    pub response_distribution: HashMap<String, u64>,
    pub hit_count: u64,
    pub total_responses: u64,
    pub hit_rate: f64,
}

pub struct DistributionAnalyzer;

impl DistributionAnalyzer {
    pub fn analyze(events: &[Event]) -> AnalysisResult {
        let mut cmd_dist = HashMap::new();
        let mut key_size_dist = HashMap::new();
        let mut value_size_dist = HashMap::new();
        let mut response_dist = HashMap::new();
        let mut hit_count = 0;
        let mut total_responses = 0;

        for event in events {
            *cmd_dist.entry(event.cmd_type).or_insert(0) += 1;
            *key_size_dist.entry(event.key_size).or_insert(0) += 1;

            if let Some(size) = event.value_size {
                *value_size_dist.entry(size).or_insert(0) += 1;
            }

            let resp_type = match event.response {
                Response::Found(_) => "found",
                Response::NotFound => "notfound",
                Response::Error => "error",
            };
            *response_dist.entry(resp_type.to_string()).or_insert(0) += 1;

            if matches!(event.response, Response::Found(_)) {
                hit_count += 1;
            }
            total_responses += 1;
        }

        let hit_rate = if total_responses > 0 {
            hit_count as f64 / total_responses as f64
        } else {
            0.0
        };

        AnalysisResult {
            total_events: events.len() as u64,
            command_distribution: cmd_dist,
            key_size_distribution: key_size_dist
                .into_iter()
                .collect::<Vec<_>>(),
            value_size_distribution: value_size_dist
                .into_iter()
                .collect::<Vec<_>>(),
            response_distribution: response_dist,
            hit_count,
            total_responses,
            hit_rate,
        }
    }
}
