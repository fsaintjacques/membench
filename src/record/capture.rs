use anyhow::{Context, Result};
use pcap::Capture;
use std::path::Path;

enum CaptureHandle {
    Live(Capture<pcap::Active>),
    Offline(Capture<pcap::Offline>),
}

pub struct PacketCapture {
    handle: CaptureHandle,
}

impl PacketCapture {
    /// Create a packet capture from a source (interface or PCAP file)
    /// Auto-detects the type by checking if source is a file
    pub fn from_source(source: &str, port: u16) -> Result<Self> {
        let handle = if Path::new(source).is_file() {
            // Open as PCAP file
            let mut cap = Capture::from_file(source)
                .context(format!("failed to open pcap file: {}", source))?;

            let filter = format!("tcp port {}", port);
            cap.filter(&filter, true)
                .context("failed to set filter")?;

            CaptureHandle::Offline(cap)
        } else {
            // Open as live interface
            let mut cap = Capture::from_device(source)
                .context(format!("failed to open device: {}", source))?
                .promisc(true)
                .snaplen(65535)
                .open()
                .context("failed to open capture")?;

            let filter = format!("tcp port {}", port);
            cap.filter(&filter, true)
                .context("failed to set filter")?;

            CaptureHandle::Live(cap)
        };

        Ok(PacketCapture { handle })
    }

    /// Legacy interface for backwards compatibility
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        Self::from_source(interface, port)
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let devices = pcap::Device::list()
            .context("failed to list devices")?;
        Ok(devices.into_iter().map(|d| d.name).collect())
    }

    pub fn next_packet(&mut self) -> Result<&[u8]> {
        let packet = match &mut self.handle {
            CaptureHandle::Live(cap) => cap.next_packet(),
            CaptureHandle::Offline(cap) => cap.next_packet(),
        }.context("failed to read packet")?;
        Ok(packet.data)
    }

    /// Check if source is finite (file) vs continuous (interface)
    pub fn is_finite(&self) -> bool {
        matches!(self.handle, CaptureHandle::Offline(_))
    }

    /// Check if source is a file (returns true) or interface (returns false)
    pub fn is_file(source: &str) -> bool {
        Path::new(source).is_file()
    }
}
