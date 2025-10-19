//! Replay infrastructure and protocol handling

use std::fmt;

pub mod analyzer;
pub mod client;
pub mod connection_task;
pub mod main;
pub mod reader;
pub mod reader_task;
pub mod streamer;

pub use analyzer::{AnalysisResult, DistributionAnalyzer};
pub use client::ReplayClient;
pub use connection_task::spawn_connection_task;
pub use main::run as run_replay;
pub use reader::ProfileReader;
pub use reader_task::{reader_task, LoopMode};
pub use streamer::ProfileStreamer;

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
