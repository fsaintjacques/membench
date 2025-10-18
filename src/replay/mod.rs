//! Replay and traffic generation logic

use std::fmt;

pub mod analyzer;
pub mod client;
pub mod generator;
pub mod reader;
pub mod main;
pub mod streamer;
pub mod connection_task;
pub mod reader_task;

pub use analyzer::{DistributionAnalyzer, AnalysisResult};
pub use client::ReplayClient;
pub use generator::TrafficGenerator;
pub use reader::ProfileReader;
pub use main::run as run_replay;
pub use streamer::ProfileStreamer;
pub use connection_task::spawn_connection_task;
pub use reader_task::{reader_task, LoopMode};

/// Protocol mode for command generation during replay
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolMode {
    /// ASCII protocol (get, set, delete, version)
    Ascii,
    /// Meta protocol (mg, ms, md, mn)
    Meta,
}

impl ProtocolMode {
    /// Parse from string (used at CLI boundary)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "ascii" => Ok(ProtocolMode::Ascii),
            "meta" => Ok(ProtocolMode::Meta),
            _ => Err(format!("Invalid protocol mode: '{}'. Use 'ascii' or 'meta'", s)),
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
