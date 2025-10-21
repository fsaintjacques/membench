//! Replay infrastructure and protocol handling

use std::fmt;
use std::str::FromStr;

pub mod analyzer;
pub mod client;
pub mod connection_task;
pub mod main;
pub mod reader;
pub mod reader_task;
pub mod stats;
mod stats_aggregator;
pub mod streamer;

pub use analyzer::{AnalysisResult, DistributionAnalyzer};
pub use client::ReplayClient;
pub use connection_task::spawn_connection_task;
pub use main::run as run_replay;
pub use reader::ProfileReader;
pub use reader_task::{reader_task, LoopMode};
pub use stats_aggregator::spawn_stats_aggregator;
pub use streamer::ProfileStreamer;

/// Protocol mode for command generation during replay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolMode {
    /// ASCII protocol (get, set, delete, version)
    Ascii,
    /// Meta protocol (mg, ms, md, mn)
    Meta,
}

impl FromStr for ProtocolMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ascii" => Ok(ProtocolMode::Ascii),
            "meta" => Ok(ProtocolMode::Meta),
            _ => Err(format!(
                "Invalid protocol mode: '{}'. Use 'ascii' or 'meta'",
                s
            )),
        }
    }
}

impl fmt::Display for ProtocolMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolMode::Ascii => write!(f, "ascii"),
            ProtocolMode::Meta => write!(f, "meta"),
        }
    }
}
