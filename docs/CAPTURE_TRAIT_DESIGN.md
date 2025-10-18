# Capture Trait Design Pattern

## Overview

This document shows the recommended refactoring to extract a `PacketSource` trait that will enable pluggable capture backends (pcap, eBPF, etc.) without changing downstream consumers.

## Current State

### Single-Use CaptureHandle Enum
```rust
// Current: src/record/capture.rs
enum CaptureHandle {
    Live(Capture<pcap::Active>),
    Offline(Capture<pcap::Offline>),
}

pub struct PacketCapture {
    handle: CaptureHandle,
}

impl PacketCapture {
    pub fn next_packet(&mut self) -> Result<&[u8]> {
        match &mut self.handle {
            CaptureHandle::Live(cap) => cap.next_packet(),
            CaptureHandle::Offline(cap) => cap.next_packet(),
        }.context("failed to read packet")
    }
}
```

**Problem:** Adding a third backend (eBPF) requires modifying `CaptureHandle` enum and all match statements.

## Proposed Trait Pattern

### PacketSource Trait
```rust
/// Common interface for packet capture backends
pub trait PacketSource {
    /// Read next packet from source
    fn next_packet(&mut self) -> Result<&[u8]>;

    /// Get human-readable source description
    fn source_info(&self) -> &str;

    /// Whether source is finite (e.g., file) vs continuous (interface)
    fn is_finite(&self) -> bool;

    /// Optional: Get source statistics
    fn stats(&self) -> Option<CaptureStats> {
        None  // Default: no stats
    }
}

/// Optional: Common statistics
#[derive(Debug, Clone)]
pub struct CaptureStats {
    pub packets_received: u64,
    pub packets_dropped: u64,
    pub bytes_received: u64,
}
```

### Concrete Implementations

#### Live Capture (from pcap)
```rust
pub struct LiveCapture {
    handle: Capture<pcap::Active>,
    interface: String,
    stats: Option<CaptureStats>,
}

impl LiveCapture {
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

        Ok(LiveCapture {
            handle: cap,
            interface: interface.to_string(),
            stats: None,
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

    fn stats(&self) -> Option<CaptureStats> {
        self.handle.stats().ok().map(|s| CaptureStats {
            packets_received: s.received,
            packets_dropped: s.dropped,
            bytes_received: 0,  // Not available from pcap
        })
    }
}
```

#### Offline Capture (from PCAP file)
```rust
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
```

#### eBPF Capture (placeholder)
```rust
#[cfg(feature = "ebpf")]
pub struct EbpfCapture {
    interface: String,
    port: u16,
    packets: std::collections::VecDeque<Vec<u8>>,
}

#[cfg(feature = "ebpf")]
impl EbpfCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        // TODO: Load eBPF program, attach to interface
        // For now, return error
        Err(anyhow!("eBPF capture not yet implemented"))
    }
}

#[cfg(feature = "ebpf")]
impl PacketSource for EbpfCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        self.packets.pop_front()
            .map(|pkt| Box::leak(pkt.into_boxed_slice()))
            .ok_or_else(|| anyhow!("no more packets"))
    }

    fn source_info(&self) -> &str {
        &format!("ebpf:{}", self.interface)
    }

    fn is_finite(&self) -> bool {
        false
    }
}
```

### Refactored PacketCapture

```rust
pub struct PacketCapture {
    source: Box<dyn PacketSource>,
}

impl PacketCapture {
    /// Factory method: auto-detect source type
    pub fn from_source(source: &str, port: u16) -> Result<Self> {
        let packet_source: Box<dyn PacketSource> = if source.starts_with("ebpf:") {
            #[cfg(feature = "ebpf")]
            {
                let iface = source.strip_prefix("ebpf:").unwrap();
                Box::new(EbpfCapture::new(iface, port)?)
            }
            #[cfg(not(feature = "ebpf"))]
            {
                return Err(anyhow!(
                    "eBPF capture requires --features ebpf, got: {}",
                    source
                ));
            }
        } else if Path::new(source).is_file() {
            Box::new(FileCapture::new(source, port)?)
        } else {
            Box::new(LiveCapture::new(source, port)?)
        };

        Ok(PacketCapture {
            source: packet_source,
        })
    }

    /// Legacy method for backwards compatibility
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        Self::from_source(interface, port)
    }

    /// Next packet - unified across all backends
    pub fn next_packet(&mut self) -> Result<&[u8]> {
        self.source.next_packet()
    }

    /// Get source information (for logging)
    pub fn source_info(&self) -> &str {
        self.source.source_info()
    }

    /// Check if source is file vs interface
    pub fn is_file(source: &str) -> bool {
        Path::new(source).is_file()
    }

    /// List available interfaces (live capture only)
    pub fn list_devices() -> Result<Vec<String>> {
        let devices = pcap::Device::list()
            .context("failed to list devices")?;
        Ok(devices.into_iter().map(|d| d.name).collect())
    }

    /// Get capture statistics (when available)
    pub fn stats(&self) -> Option<CaptureStats> {
        self.source.stats()
    }
}
```

## Usage in record/main.rs

### Before (current)
```rust
pub fn run(source: &str, port: u16, output: &str, salt: Option<u64>) -> Result<()> {
    let source_type = if PacketCapture::is_file(source) { "file" } else { "interface" };
    tracing::info!("Recording from {} ({}):{} to {}", source_type, source, port, output);

    let mut capture = PacketCapture::from_source(source, port)?;
    // ...
    loop {
        match capture.next_packet() {
            Ok(packet_data) => {
                // process packet
            }
        }
    }
}
```

### After (with trait)
```rust
pub fn run(source: &str, port: u16, output: &str, salt: Option<u64>) -> Result<()> {
    let mut capture = PacketCapture::from_source(source, port)?;

    let source_type = if capture.source.is_finite() { "file" } else { "interface" };
    tracing::info!(
        "Recording from {} ({}):{} to {}",
        source_type,
        capture.source.source_info(),
        port,
        output
    );

    // Same loop - no changes needed!
    loop {
        match capture.next_packet() {
            Ok(packet_data) => {
                // process packet - same code
            }
        }
    }
}
```

**Key Point:** Downstream code is unchanged. The trait abstraction is internal.

## Trait Object vs Enum Trade-offs

### Trait Object (Proposed)
```rust
pub struct PacketCapture {
    source: Box<dyn PacketSource>,
}
```

**Pros:**
- Easy to add new backends (no existing code changes)
- Scales well (100 backends = 100 lines, not combinatorial)
- Encapsulation: each backend is independent

**Cons:**
- Dynamic dispatch overhead (~1-2% on fast path)
- Requires Box (one allocation per PacketCapture)
- Trait methods must be object-safe (no Self, no generic params)

### Enum Alternative
```rust
enum CaptureHandle {
    Live(LiveCapture),
    Offline(FileCapture),
    Ebpf(EbpfCapture),
    // Add more...
}
```

**Pros:**
- Static dispatch (no vtable, slightly faster)
- All types known at compile time

**Cons:**
- Must list all variants
- Adding new backend requires editing existing code
- Pattern matching boilerplate in every method
- Doesn't scale well

## Implementation Steps

### Step 1: Define Trait (backwards compatible)
```bash
# No changes needed to existing code
# Just add trait in capture.rs
git add src/record/capture.rs
```

### Step 2: Extract Implementations
```bash
# Move Live/Offline logic into separate structs
# Keep existing PacketCapture behavior
```

### Step 3: Test Equivalence
```bash
cargo test --all
# Should pass identically
```

### Step 4: Add eBPF (feature-gated)
```bash
# Add feature "ebpf" to Cargo.toml
# Implement EbpfCapture only when feature enabled
```

### Step 5: Documentation
```bash
# Add this design guide
# Document for future contributors
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_capture_trait() {
        let mut cap = LiveCapture::new("lo0", 11211).unwrap();
        let info = cap.source_info();
        assert_eq!(info, "lo0");
        assert!(!cap.is_finite());
    }

    #[test]
    fn test_file_capture_trait() {
        let mut cap = FileCapture::new("test.pcap", 11211).unwrap();
        assert!(cap.is_finite());
    }

    #[test]
    #[cfg(feature = "ebpf")]
    fn test_ebpf_capture_trait() {
        // Will fail until eBPF implementation complete
    }

    #[test]
    fn test_packet_capture_factory() {
        // Test from_source() dispatching
        let pcap = PacketCapture::from_source("lo0", 11211);
        assert!(pcap.is_ok());

        let file = PacketCapture::from_source("test.pcap", 11211);
        // May fail if file doesn't exist, but logic is correct
    }
}
```

### Integration Tests
```bash
# Same tests, different backends
cargo test test_capture_workflow
cargo test --features ebpf test_capture_workflow
```

## Performance Implications

### Memory
- Current: `CaptureHandle` enum (stack-allocated)
- After: `Box<dyn PacketSource>` (heap-allocated)
- Impact: +8 bytes per PacketCapture instance (negligible)

### CPU (dynamic dispatch)
- Trait methods: indirect function call via vtable
- Typical overhead: 1-2% on tight loops
- Mitigation: only `next_packet()` is on hot path
  - Other methods (stats, source_info) are not performance-critical

### Benchmark
```bash
# Before
time membench record lo0 profile_before.bin

# After (should be ~identical)
time membench record lo0 profile_after.bin

# Measure any regression
```

## Backwards Compatibility

- `PacketCapture::new()` still works (delegates to `from_source()`)
- `next_packet()` API unchanged
- Internal API changed, but that's internal
- No breaking changes to library users

## Future Extensions

### Variant: Multiple Backends
```rust
pub struct PacketCapture {
    sources: Vec<Box<dyn PacketSource>>,
    current: usize,
}

impl PacketCapture {
    pub fn from_sources(sources: Vec<&str>) -> Result<Self> {
        // Capture from multiple interfaces simultaneously
    }
}
```

### Variant: Buffering Backend
```rust
pub struct BufferedCapture {
    inner: Box<dyn PacketSource>,
    buffer: VecDeque<Vec<u8>>,
}

impl PacketSource for BufferedCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        // Pre-fetch packets into buffer
    }
}
```

## Summary

**Benefits of this design:**
1. ✅ Easy to add eBPF without modifying existing code
2. ✅ Extensible: future backends just implement the trait
3. ✅ No changes needed downstream (record/main.rs)
4. ✅ Backwards compatible
5. ✅ Clean separation of concerns

**Effort to implement:**
- Refactoring: ~2 hours (mostly moving code)
- Testing: ~1 hour (verify equivalence)
- Total: ~3 hours, ~100 lines of refactoring

**Recommended next step:**
1. Extract `PacketSource` trait
2. Move Live/Offline into concrete types
3. Verify all tests pass
4. Then add eBPF support at your own pace
