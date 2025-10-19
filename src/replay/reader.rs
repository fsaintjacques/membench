use crate::profile::{Event, ProfileMetadata};
use anyhow::Result;
use std::fs;

pub struct ProfileReader {
    metadata: ProfileMetadata,
    events: Vec<Event>,
}

impl ProfileReader {
    pub fn new(path: &str) -> Result<Self> {
        let data = fs::read(path)?;

        if data.len() < 4 {
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

        // Metadata format: [metadata_data][metadata_len:2 bytes][end_marker:4 bytes]
        // So read metadata_len from before the end marker
        if end_marker_pos < 2 {
            return Err(anyhow::anyhow!("file too small for metadata"));
        }

        let metadata_len_pos = end_marker_pos - 2;
        let metadata_len =
            u16::from_le_bytes([data[metadata_len_pos], data[metadata_len_pos + 1]]) as usize;

        if metadata_len_pos < metadata_len {
            return Err(anyhow::anyhow!("metadata length exceeds file size"));
        }

        let metadata_start = metadata_len_pos - metadata_len;
        let metadata_bytes = &data[metadata_start..metadata_len_pos];
        let metadata: ProfileMetadata = bincode::deserialize(metadata_bytes)?;

        // Read events from beginning up to metadata
        let mut events = Vec::new();
        let mut offset = 0;

        while offset < metadata_start {
            if offset + 2 > metadata_start {
                break;
            }

            let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            if offset + len > metadata_start {
                break;
            }

            let event_bytes = &data[offset..offset + len];
            let event: Event = bincode::deserialize(event_bytes)?;
            events.push(event);
            offset += len;
        }

        Ok(ProfileReader { metadata, events })
    }

    pub fn metadata(&self) -> &ProfileMetadata {
        &self.metadata
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }
}
