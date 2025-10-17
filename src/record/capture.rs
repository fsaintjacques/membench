use anyhow::{Context, Result};
use pcap::Capture;

pub struct PacketCapture {
    handle: Capture<pcap::Active>,
    port: u16,
}

impl PacketCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        let mut cap = Capture::from_device(interface)
            .context("failed to open device")?
            .promisc(true)
            .snaplen(65535)
            .open()
            .context("failed to open capture")?;

        let filter = format!("tcp port {}", port);
        cap.filter(&filter, true)
            .context("failed to set filter")?;

        Ok(PacketCapture { handle: cap, port })
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let devices = pcap::Device::list()
            .context("failed to list devices")?;
        Ok(devices.into_iter().map(|d| d.name).collect())
    }

    pub fn next_packet(&mut self) -> Result<&[u8]> {
        let packet = self.handle
            .next_packet()
            .context("failed to read packet")?;
        Ok(packet.data)
    }
}
