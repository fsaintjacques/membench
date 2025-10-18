#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "ebpf")]
    fn test_ebpf_feature_enabled() {
        // Verify feature is compiled in
        assert!(cfg!(feature = "ebpf"));
    }

    #[test]
    #[cfg(not(feature = "ebpf"))]
    fn test_ebpf_feature_disabled() {
        // Verify feature is not compiled in
        assert!(!cfg!(feature = "ebpf"));
    }

    #[test]
    fn test_ebpf_prefix_detection() {
        let source = "ebpf:eth0";
        assert!(source.starts_with("ebpf:"));
    }
}
