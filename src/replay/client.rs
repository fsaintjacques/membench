use anyhow::Result;
use std::net::TcpStream;
use std::io::{Read, Write};
use crate::profile::{Event, CommandType};

pub struct ReplayClient {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl ReplayClient {
    pub fn new(target: &str, buf_capacity: usize) -> Result<Self> {
        let stream = TcpStream::connect(target)?;
        Ok(ReplayClient {
            stream,
            buffer: vec![0u8; buf_capacity],
        })
    }

    pub fn send_command(&mut self, event: &Event) -> Result<()> {
        let cmd = self.build_command_string(event);
        self.stream.write_all(cmd.as_bytes())?;
        Ok(())
    }

    pub fn read_response(&mut self) -> Result<Vec<u8>> {
        let n = self.stream.read(&mut self.buffer)?;
        Ok(self.buffer[..n].to_vec())
    }

    fn build_command_string(&self, event: &Event) -> String {
        match event.cmd_type {
            CommandType::Get => {
                format!("mg {} v\r\n", "key")  // Placeholder key
            }
            CommandType::Set => {
                let size = event.value_size.unwrap_or(0);
                format!("ms {} {}\r\n{}\r\n", "key", size, "value")
            }
            CommandType::Delete => {
                format!("md {}\r\n", "key")
            }
            CommandType::Noop => {
                "mn\r\n".to_string()
            }
        }
    }
}
