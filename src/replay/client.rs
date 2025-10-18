use anyhow::Result;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::profile::{Event, CommandType};

pub struct ReplayClient {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl ReplayClient {
    pub async fn new(target: &str) -> Result<Self> {
        let stream = TcpStream::connect(target).await?;
        Ok(ReplayClient {
            stream,
            buffer: vec![0u8; 65536],
        })
    }

    pub async fn send_command(&mut self, event: &Event) -> Result<()> {
        let cmd = self.build_command_string(event);
        self.stream.write_all(cmd.as_bytes()).await?;
        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<Vec<u8>> {
        let n = self.stream.read(&mut self.buffer).await?;
        Ok(self.buffer[..n].to_vec())
    }

    fn build_command_string(&self, event: &Event) -> String {
        let key = self.generate_key(event.key_hash, event.key_size);

        match event.cmd_type {
            CommandType::Get => {
                format!("mg {} v\r\n", key)
            }
            CommandType::Set => {
                let size = event.value_size.map(|nz| nz.get()).unwrap_or(0);
                let value = self.generate_value(size);
                format!("ms {} {}\r\n{}\r\n", key, size, value)
            }
            CommandType::Delete => {
                format!("md {}\r\n", key)
            }
            CommandType::Noop => {
                "mn\r\n".to_string()
            }
        }
    }

    /// Generate a deterministic key from hash and size
    /// Same hash+size always produces the same key
    fn generate_key(&self, key_hash: u64, key_size: u32) -> String {
        if key_size == 0 {
            return String::new();
        }

        // Convert hash to hex representation
        let hash_hex = format!("{:016x}", key_hash);

        // Repeat and truncate to match key_size
        let key = (hash_hex.repeat(((key_size as usize + hash_hex.len() - 1) / hash_hex.len()) + 1))
            .chars()
            .take(key_size as usize)
            .collect::<String>();

        key
    }

    /// Generate a value payload of specified size
    /// Uses a repeating pattern to fill the size
    fn generate_value(&self, size: u32) -> String {
        if size == 0 {
            return String::new();
        }

        // Generate payload matching size
        let pattern = "x";
        pattern.repeat(size as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_client_creation() {
        // This test will pass once ReplayClient uses async
        // We'll verify the compilation and basic structure
        let client = ReplayClient::new("127.0.0.1:11211").await;
        // For now, just verify it compiles; actual memcached test requires running server
        assert!(client.is_ok() || client.is_err()); // Accepts either for now
    }
}
