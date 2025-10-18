use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum CommandType {
    Get,
    Set,
    Delete,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Flags {
    bits: u32,
}

impl Flags {
    pub fn empty() -> Self {
        Flags { bits: 0 }
    }

    pub fn with_quiet(mut self) -> Self {
        self.bits |= 1 << 0;
        self
    }

    pub fn has_quiet(&self) -> bool {
        (self.bits & (1 << 0)) != 0
    }

    pub fn with_value(mut self) -> Self {
        self.bits |= 1 << 1;
        self
    }

    pub fn has_value(&self) -> bool {
        (self.bits & (1 << 1)) != 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: u64,
    pub conn_id: u32,
    pub cmd_type: CommandType,
    pub key_hash: u64,
    pub key_size: u32,
    pub value_size: Option<u32>,
    pub flags: Flags,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub magic: u32,
    pub version: u8,
    pub total_events: u64,
    pub time_range: (u64, u64),
    pub unique_connections: u32,
    pub command_distribution: HashMap<CommandType, u64>,
}

impl ProfileMetadata {
    pub fn new() -> Self {
        ProfileMetadata {
            magic: 0xDEADBEEF,
            version: 1,
            total_events: 0,
            time_range: (0, 0),
            unique_connections: 0,
            command_distribution: HashMap::new(),
        }
    }
}
