use anyhow::Result;
use std::fs::File;
use std::io::{BufWriter, Write};
use crate::profile::{Event, ProfileMetadata};
use std::collections::HashSet;

pub struct ProfileWriter {
    file: BufWriter<File>,
    metadata: ProfileMetadata,
    events_written: u64,
    first_timestamp: Option<u64>,
    last_timestamp: Option<u64>,
    connections: HashSet<u32>,
}

impl ProfileWriter {
    pub fn new(path: &str) -> Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        let metadata = ProfileMetadata::new();

        Ok(ProfileWriter {
            file: writer,
            metadata,
            events_written: 0,
            first_timestamp: None,
            last_timestamp: None,
            connections: HashSet::new(),
        })
    }

    pub fn write_event(&mut self, event: &Event) -> Result<()> {
        let encoded = bincode::serialize(event)?;

        // Write event with u16 length prefix
        self.file.write_all(&(encoded.len() as u16).to_le_bytes())?;
        self.file.write_all(&encoded)?;

        self.events_written += 1;
        self.connections.insert(event.conn_id);

        if self.first_timestamp.is_none() {
            self.first_timestamp = Some(event.timestamp);
        }
        self.last_timestamp = Some(event.timestamp);

        *self.metadata.command_distribution
            .entry(event.cmd_type)
            .or_insert(0) += 1;

        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.metadata.total_events = self.events_written;
        self.metadata.unique_connections = self.connections.len() as u32;

        if let (Some(first), Some(last)) = (self.first_timestamp, self.last_timestamp) {
            self.metadata.time_range = (first, last);
        }

        // Write metadata: data first, then length prefix
        let encoded_metadata = bincode::serialize(&self.metadata)?;
        self.file.write_all(&encoded_metadata)?;
        self.file.write_all(&(encoded_metadata.len() as u16).to_le_bytes())?;

        // Write end marker: magic number so we know where metadata ends
        self.file.write_all(&0xDEADBEEFu32.to_le_bytes())?;

        self.file.flush()?;
        Ok(())
    }
}
