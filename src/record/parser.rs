use anyhow::{anyhow, Result};
use crate::profile::{CommandType, Response, Flags};

pub struct ParsedCommand {
    pub cmd_type: CommandType,
    pub key_range: std::ops::Range<usize>,
    pub value_size: Option<u32>,
    pub flags: Flags,
}

pub struct ParsedResponse {
    pub resp: Response,
    pub consumed: usize,
}

pub struct MemcacheParser;

impl MemcacheParser {
    pub fn new() -> Self {
        MemcacheParser
    }

    pub fn parse_command<'a>(&self, input: &'a [u8]) -> Result<(ParsedCommand, &'a [u8])> {
        let line_end = input.iter().position(|&b| b == b'\n')
            .ok_or(anyhow!("no newline"))?;
        let line = &input[..line_end - 1]; // exclude \r
        let rest = &input[line_end + 1..];

        let parts: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
        if parts.is_empty() {
            return Err(anyhow!("empty command"));
        }

        let cmd = std::str::from_utf8(parts[0])?.to_lowercase();
        let cmd_type = match cmd.as_str() {
            "get" => CommandType::Get,
            "mg" => CommandType::Get,      // Meta protocol
            "set" => CommandType::Set,
            "ms" => CommandType::Set,      // Meta protocol
            "delete" => CommandType::Delete,
            "md" => CommandType::Delete,   // Meta protocol
            "noop" => CommandType::Noop,
            "mn" => CommandType::Noop,     // Meta protocol
            _ => return Err(anyhow!("unknown command: {}", cmd)),
        };

        if parts.len() < 2 {
            return Err(anyhow!("missing key"));
        }

        let key_start = parts[0].len() + 1;
        let key_end = key_start + parts[1].len();

        let value_size = if cmd_type == CommandType::Set && parts.len() > 2 {
            Some(std::str::from_utf8(parts[2])?.parse()?)
        } else {
            None
        };

        Ok((ParsedCommand {
            cmd_type,
            key_range: key_start..key_end,
            value_size,
            flags: Flags::empty(),
        }, rest))
    }

    pub fn parse_response(&self, input: &[u8]) -> Result<ParsedResponse> {
        let line_end = input.iter().position(|&b| b == b'\n')
            .ok_or(anyhow!("no newline"))?;
        let line = &input[..line_end - 1];

        let parts: Vec<&[u8]> = line.split(|&b| b == b' ').collect();
        if parts.is_empty() {
            return Err(anyhow!("empty response"));
        }

        let resp_type = std::str::from_utf8(parts[0])?;
        let response = match resp_type {
            "VA" => {
                let size: u32 = std::str::from_utf8(parts[1])?.parse()?;
                Response::Found(size)
            }
            "EN" => Response::NotFound,
            "EX" => Response::Error,
            "HD" => Response::Found(0),
            _ => return Err(anyhow!("unknown response: {}", resp_type)),
        };

        Ok(ParsedResponse {
            resp: response,
            consumed: line_end + 1,
        })
    }
}
