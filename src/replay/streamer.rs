use anyhow::Result;
use crate::profile::Event;
use std::fs::File;
use std::io::{BufReader, Seek};

/// Streams events from a profile file one at a time without loading all into memory
pub struct ProfileStreamer {
    file: File,
}

impl ProfileStreamer {
    pub fn new(path: &str) -> Result<Self> {
        let file = File::open(path)?;
        Ok(ProfileStreamer { file })
    }

    /// Get the next event from the stream, or None if end of file
    pub fn next(&mut self) -> Result<Option<Event>> {
        use bincode::Options;

        let mut reader = BufReader::new(&mut self.file);

        // Try to deserialize one event
        match bincode::options().deserialize_from(&mut reader) {
            Ok(event) => Ok(Some(event)),
            Err(e) if matches!(*e, bincode::ErrorKind::Io(_)) => {
                // EOF or actual IO error - check if EOF
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Reset to beginning of file
    pub fn reset(&mut self) -> Result<()> {
        self.file.seek(std::io::SeekFrom::Start(0))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streamer_creation() {
        // Will test once we have a sample profile file
        // For now, verify the structure compiles
    }
}
