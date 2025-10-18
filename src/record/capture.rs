use anyhow::{Context, Result};
use pcap::Capture;
use std::path::Path;

/// Common interface for packet capture backends
pub trait PacketSource {
    /// Read next packet from source
    fn next_packet(&mut self) -> Result<&[u8]>;

    /// Get human-readable source description (interface name or file path)
    fn source_info(&self) -> &str;

    /// Whether source is finite (file) vs continuous (interface)
    fn is_finite(&self) -> bool;

    /// Optional: Get capture statistics (when available)
    fn stats(&mut self) -> Option<CaptureStats> {
        None  // Default: no stats
    }
}

/// Optional statistics from capture
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub bytes_received: u64,
}

/// Live network interface capture
pub struct LiveCapture {
    handle: Capture<pcap::Active>,
    interface: String,
}

impl LiveCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        let mut cap = Capture::from_device(interface)
            .context(format!("failed to open device: {}", interface))?
            .promisc(true)
            .snaplen(65535)
            .open()
            .context("failed to open capture")?;

        let filter = format!("tcp port {}", port);
        cap.filter(&filter, true)
            .context("failed to set filter")?;

        Ok(LiveCapture {
            handle: cap,
            interface: interface.to_string(),
        })
    }
}

impl PacketSource for LiveCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        self.handle.next_packet()
            .context("failed to read packet")
            .map(|pkt| pkt.data)
    }

    fn source_info(&self) -> &str {
        &self.interface
    }

    fn is_finite(&self) -> bool {
        false  // Network interface is continuous
    }

    fn stats(&mut self) -> Option<CaptureStats> {
        self.handle.stats().ok().map(|s| CaptureStats {
            packets_received: s.received as u64,
            packets_dropped: s.dropped as u64,
            bytes_received: 0,
        })
    }
}

/// PCAP file capture (offline)
pub struct FileCapture {
    handle: Capture<pcap::Offline>,
    path: String,
}

impl FileCapture {
    pub fn new(path: &str, port: u16) -> Result<Self> {
        let mut cap = Capture::from_file(path)
            .context(format!("failed to open pcap file: {}", path))?;

        let filter = format!("tcp port {}", port);
        cap.filter(&filter, true)
            .context("failed to set filter")?;

        Ok(FileCapture {
            handle: cap,
            path: path.to_string(),
        })
    }
}

impl PacketSource for FileCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        self.handle.next_packet()
            .context("failed to read packet")
            .map(|pkt| pkt.data)
    }

    fn source_info(&self) -> &str {
        &self.path
    }

    fn is_finite(&self) -> bool {
        true  // File has end
    }
}

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
