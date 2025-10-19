use crate::profile::Event;
use anyhow::Result;
use std::fs::File;
use std::io::Read;

pub struct ProfileStreamer {
    data: Vec<u8>,
    event_end_offset: usize,
    current_offset: usize,
}

impl ProfileStreamer {
    pub fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        if data.len() < 6 {
            return Err(anyhow::anyhow!("file too small"));
        }

        // Last 4 bytes are the end marker
        let end_marker_pos = data.len() - 4;
        let end_marker = u32::from_le_bytes([
            data[end_marker_pos],
            data[end_marker_pos + 1],
            data[end_marker_pos + 2],
            data[end_marker_pos + 3],
        ]);

        if end_marker != 0xDEADBEEF {
            return Err(anyhow::anyhow!("invalid file format: missing end marker"));
        }

        // Read metadata length
        let metadata_len_pos = end_marker_pos - 2;
        let metadata_len =
            u16::from_le_bytes([data[metadata_len_pos], data[metadata_len_pos + 1]]) as usize;

        let event_end_offset = metadata_len_pos - metadata_len;

        Ok(ProfileStreamer {
            data,
            event_end_offset,
            current_offset: 0,
        })
    }

    pub fn next_event(&mut self) -> Result<Option<Event>> {
        // Check if we've reached the metadata section
        if self.current_offset >= self.event_end_offset {
            return Ok(None);
        }

        // Check if we have room for length prefix
        if self.current_offset + 2 > self.event_end_offset {
            return Ok(None);
        }

        // Read length prefix
        let len = u16::from_le_bytes([
            self.data[self.current_offset],
            self.data[self.current_offset + 1],
        ]) as usize;
        self.current_offset += 2;

        // Check if we have room for event data
        if self.current_offset + len > self.event_end_offset {
            return Err(anyhow::anyhow!("event data exceeds file boundary"));
        }

        // Deserialize event
        let event_bytes = &self.data[self.current_offset..self.current_offset + len];
        let event: Event = bincode::deserialize(event_bytes)?;
        self.current_offset += len;

        Ok(Some(event))
    }

    pub fn reset(&mut self) -> Result<()> {
        self.current_offset = 0;
        Ok(())
    }
}
