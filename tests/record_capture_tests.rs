#[cfg(test)]
mod tests {
    use membench::record::PacketCapture;

    #[test]
    fn test_capture_interface_creation() {
        // We can't fully test libpcap without real network setup,
        // but we can test the interface exists and responds to basic calls
        let devices = PacketCapture::list_devices();
        // Just verify we can list devices without panic
        assert!(devices.is_ok());
    }
}
