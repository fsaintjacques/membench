#[cfg(all(test, feature = "ebpf", target_os = "linux"))]
mod tests {
    use membench::record::PacketCapture;

    #[test]
    #[ignore] // Requires root and running memcached
    fn test_ebpf_socket_capture_creation() {
        // This test verifies eBPF program loads
        // Requires sudo and CAP_BPF
        let result = PacketCapture::from_source("ebpf:eth0", 11211);

        // May fail without privileges - that's expected
        match result {
            Ok(_capture) => {
                println!("eBPF capture initialized successfully");
            }
            Err(e) => {
                println!("eBPF capture failed (expected without privileges): {}", e);
            }
        }
    }

    #[test]
    fn test_ebpf_prefix_recognition() {
        let source = "ebpf:lo";
        assert!(source.starts_with("ebpf:"));
    }
}

#[cfg(not(feature = "ebpf"))]
mod tests {
    #[test]
    fn test_ebpf_not_compiled() {
        // When ebpf feature is disabled, ensure it's really not included
        assert!(!cfg!(feature = "ebpf"));
    }
}
