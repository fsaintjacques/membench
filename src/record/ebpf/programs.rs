//! eBPF program and userspace integration

use anyhow::Result;
use crate::record::capture::CaptureStats;
use crate::record::capture::PacketSource;

/// eBPF packet capture using TC ingress hook
pub struct EbpfCapture {
    interface: String,
    port: u16,
}

impl EbpfCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        // TODO: Initialize eBPF program
        Ok(EbpfCapture {
            interface: interface.to_string(),
            port,
        })
    }
}

impl PacketSource for EbpfCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        todo!("Implement eBPF packet reading")
    }

    fn source_info(&self) -> &str {
        &self.interface
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn stats(&mut self) -> Option<CaptureStats> {
        None
    }
}
