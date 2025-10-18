# eBPF Integration: Design & Implementation Strategy

## Current Capture Abstraction

### PacketCapture Design
```rust
pub struct PacketCapture {
    handle: CaptureHandle,
}

enum CaptureHandle {
    Live(Capture<pcap::Active>),        // Kernel libpcap
    Offline(Capture<pcap::Offline>),    // PCAP file playback
}

impl PacketCapture {
    pub fn from_source(source: &str, port: u16) -> Result<Self> { ... }
    pub fn next_packet(&mut self) -> Result<&[u8]> { ... }
    pub fn is_file(source: &str) -> bool { ... }
}
```

### Current API Contract
- `from_source(source: &str, port: u16)` → auto-detects file vs interface
- `next_packet() → Result<&[u8]>` → returns packet payload (raw bytes)
- Unified interface: same code path for live and offline
- BPF filter applied: `tcp port 11211` (port filtering)

## Capture Trait Abstraction

### Proposed Trait Interface
```rust
pub trait PacketSource {
    /// Read next packet from source
    fn next_packet(&mut self) -> Result<&[u8]>;

    /// Get source information for logging
    fn source_info(&self) -> &str;

    /// Check if this source is finite (e.g., file) or continuous (interface)
    fn is_finite(&self) -> bool;
}

impl PacketSource for LiveCapture { ... }
impl PacketSource for FileCapture { ... }
impl PacketSource for EbpfCapture { ... }

pub struct PacketCapture {
    source: Box<dyn PacketSource>,
}
```

### Refactoring for eBPF
```rust
// New wrapper types
pub struct LiveCapture {
    handle: Capture<pcap::Active>,
}

pub struct FileCapture {
    handle: Capture<pcap::Offline>,
}

pub struct EbpfCapture {
    events: Vec<Vec<u8>>,  // Buffer from BPF maps
    index: usize,
    source_info: String,
}

// Factory method
impl PacketCapture {
    pub fn from_source(source: &str, port: u16) -> Result<Self> {
        let capture_impl: Box<dyn PacketSource> = if source.starts_with("ebpf:") {
            // eBPF mode: "ebpf:eth0" or "ebpf:*"
            let iface = source.strip_prefix("ebpf:").unwrap();
            Box::new(EbpfCapture::new(iface, port)?)
        } else if Path::new(source).is_file() {
            // File mode
            Box::new(FileCapture::new(source, port)?)
        } else {
            // Live interface mode
            Box::new(LiveCapture::new(source, port)?)
        };
        Ok(PacketCapture { source: capture_impl })
    }
}
```

## eBPF Implementation Approach

### Why eBPF for Capture?

**Advantages:**
1. **Lower overhead** than userspace libpcap
   - Kernel-space filtering
   - Reduced context switches
   - Less data copied to userspace
2. **Rich filtering** on packet content
   - Parse TCP headers (detect connection start/end)
   - Extract memcache protocol markers
3. **Early aggregation** possible
   - Count events by type
   - Aggregate statistics in kernel

**Tradeoffs:**
1. **Complexity**: eBPF code + userspace integration
2. **Portability**: Linux only (libpcap works on macOS, Windows)
3. **Safety**: eBPF verifier constraints
4. **Debugging**: Harder to debug kernel-space code

### eBPF Module Architecture

```
membench/
├── src/
│   └── record/
│       ├── capture.rs              # PacketSource trait, impls
│       └── ebpf/
│           ├── mod.rs              # eBPF module exports
│           ├── capture.rs          # EbpfCapture struct
│           └── kern/
│               ├── lib.rs          # Kernel eBPF programs
│               ├── build.rs        # eBPF build script
│               └── vmlinux.rs      # Auto-generated (btf)
└── Cargo.toml
```

### Implementation Stages

#### Stage 1: Trait Abstraction (Non-eBPF)
**Goal**: Make capture backend pluggable without eBPF yet
- Refactor current code into `LiveCapture`, `FileCapture` impls
- Create `PacketSource` trait
- Update `PacketCapture` to use trait objects
- No eBPF code yet, just cleaner abstractions
- **Benefit**: Simpler review, easier testing

#### Stage 2: Basic eBPF Capture (Userspace + Kernel)
**Goal**: Add minimal eBPF implementation
- Create `EbpfCapture` struct
- Implement basic BPF program (packet filtering only)
- Read packets from BPF perf buffer
- Full packet pass-through (no in-kernel aggregation)
- Same downstream processing
- **Benefit**: Proof-of-concept, measure overhead

#### Stage 3: In-Kernel Parsing (Optional)
**Goal**: Move packet→event conversion to kernel
- Parse TCP headers in eBPF
- Extract memcache command type
- Generate Event structs in kernel
- Store in BPF map
- Userspace reads pre-parsed events
- **Benefit**: Max performance gain, but higher complexity

### eBPF Program Sketch (Stage 1)

```c
// vmlinux.h (auto-generated from kernel BTF)
#include "vmlinux.h"

SEC("tc/ingress")
int memcache_capture(struct __sk_buff *skb) {
    // Parse Ethernet header
    void *data = (void *)(long)skb->data;
    void *data_end = (void *)(long)skb->data_end;

    // Parse IP header
    struct iphdr *ip = data + ETH_HLEN;
    if ((void *)(ip + 1) > data_end)
        return TC_ACT_OK;

    // Filter: TCP port 11211 only
    if (ip->protocol != IPPROTO_TCP)
        return TC_ACT_OK;

    struct tcphdr *tcp = (void *)ip + (ip->ihl << 2);
    if ((void *)(tcp + 1) > data_end)
        return TC_ACT_OK;

    __u16 sport = ntohs(tcp->source);
    __u16 dport = ntohs(tcp->dest);
    if (dport != 11211 && sport != 11211)
        return TC_ACT_OK;

    // Send packet to perf buffer
    bpf_perf_event_output(skb, &events, BPF_F_CURRENT_CPU,
                          skb->len, data, skb->len);

    return TC_ACT_OK;
}
```

### Userspace Integration

```rust
pub struct EbpfCapture {
    // Use aya or libbpf-rs
    ebpf: Ebpf,
    events_buf: Ring,  // From perf buffer
}

impl EbpfCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        // Load eBPF program
        let ebpf = Ebpf::load(ebpf::programs())?;

        // Attach to TC ingress
        ebpf.tc_qdisc_add_clsact(interface)?;
        ebpf.tc_filter_add(interface, ...)?;

        // Open perf buffer for events
        let events_buf = ebpf.events().open()?;

        Ok(EbpfCapture { ebpf, events_buf })
    }
}

impl PacketSource for EbpfCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        // Read from perf buffer (returns packet bytes)
        let event = self.events_buf.read()?;
        Ok(&event.packet[..])
    }

    fn source_info(&self) -> &str { "ebpf" }
    fn is_finite(&self) -> bool { false }
}
```

## eBPF Dependencies

### Crate Options

**Option A: aya (Most Rust-idiomatic)**
```toml
[dependencies]
aya = "0.13"
aya-ebpf = "0.13"

[build-dependencies]
aya-build = "0.13"
```
- Pros: Rust-first, good ergonomics, active development
- Cons: Smaller ecosystem than libbpf
- Best for: New projects, pure-Rust preference

**Option B: libbpf-rs (libbpf wrapper)**
```toml
[dependencies]
libbpf-rs = "0.21"

[build-dependencies]
libbpf-cargo = "0.21"
```
- Pros: battle-tested libbpf backend, more tools
- Cons: Wrapper overhead, C interop
- Best for: Production, C ecosystem integration

**Option C: Hybrid (Feature flag)**
```toml
[dependencies]
aya = { version = "0.13", optional = true }
libbpf-rs = { version = "0.21", optional = true }

[features]
ebpf = ["aya"]          # Default eBPF backend
ebpf-libbpf = ["libbpf-rs"]
```

### Kernel Requirements
- Linux 5.8+ (BPF_LINK support)
- BPF programs require CAP_BPF + CAP_PERFMON (or CAP_SYS_ADMIN)
- BPF Type Format (BTF) support for vmlinux.h auto-generation

## Compatibility & Fallback

### Detection & Graceful Fallback
```rust
pub fn from_source(source: &str, port: u16) -> Result<Self> {
    if source.starts_with("ebpf:") {
        // User explicitly requested eBPF
        match EbpfCapture::new(source, port) {
            Ok(cap) => return Ok(PacketCapture::from_impl(cap)),
            Err(e) => {
                // eBPF failed - could try fallback
                eprintln!("eBPF capture failed: {}. Trying pcap...", e);
                // Fallback to pcap? Or error out?
                return Err(e);
            }
        }
    }
    // Existing Live/Offline logic...
}
```

### Platform Support
```rust
#[cfg(target_os = "linux")]
use ebpf::EbpfCapture;

#[cfg(not(target_os = "linux"))]
fn load_ebpf(_: &str) -> Result<EbpfCapture> {
    Err(anyhow!("eBPF capture only supported on Linux"))
}
```

## Testing eBPF Implementation

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_ebpf_capture_load() {
        let result = EbpfCapture::new("lo0", 11211);
        // May fail if kernel is old or CAP_BPF missing
    }

    #[test]
    fn test_pcap_fallback() {
        // Ensure pcap still works without eBPF
        let cap = PacketCapture::from_source("lo0", 11211);
        assert!(cap.is_ok());
    }
}
```

### Integration Tests
```bash
# Test with eBPF (requires Linux + CAP_BPF)
cargo test --features ebpf

# Test without eBPF (pcap only)
cargo test

# Compare throughput
time membench record "ebpf:eth0" profile_ebpf.bin
time membench record eth0 profile_pcap.bin
```

## Feature Flag Configuration

### Cargo.toml
```toml
[features]
default = []
ebpf = ["aya"]  # Optional eBPF support
ebpf-libbpf = ["libbpf-rs"]
all = ["ebpf"]

[dependencies]
aya = { version = "0.13", optional = true }
aya-ebpf = { version = "0.13", optional = true }
```

### Build Script
```bash
# Build without eBPF (current behavior)
cargo build

# Build with eBPF support
cargo build --features ebpf

# eBPF program only compiled if feature enabled
# Binary size: ~1-2 MB added for eBPF loader
```

## Performance Expectations

### Benchmark Scenarios
| Scenario | Current (libpcap) | eBPF (Stage 1) | eBPF (Stage 3) |
|----------|------------------|----------------|----------------|
| Packet capture rate | ~50k-100k pps | ~70k-150k pps | ~200k+ pps |
| CPU overhead | ~30% | ~15% | ~5% |
| Memory (1M packets) | ~51 MB | ~51 MB | ~10 MB |
| Latency P99 | ~5ms | ~2ms | ~500µs |

**Why different?**
- Stage 1: Kernel filtering (less data copied)
- Stage 3: No userspace parsing (less CPU)

## Implementation Roadmap

### Phase 1: Trait Abstraction (Week 1)
- [ ] Create `PacketSource` trait
- [ ] Refactor into `LiveCapture`, `FileCapture`
- [ ] Update `PacketCapture` struct
- [ ] All tests pass (no eBPF yet)
- [ ] Benchmark: should be identical to current

### Phase 2: eBPF Skeleton (Week 2)
- [ ] Add `aya` dependency
- [ ] Create `EbpfCapture` struct (stub)
- [ ] Implement trait for `EbpfCapture`
- [ ] Unit tests (load, attach, etc.)
- [ ] Build script for eBPF programs

### Phase 3: Basic eBPF Program (Week 3)
- [ ] Write TC ingress eBPF program
- [ ] Filter by TCP port 11211
- [ ] Send packets to perf buffer
- [ ] Integration test with real packet capture
- [ ] Benchmark vs libpcap

### Phase 4: In-Kernel Parsing (Week 4, Optional)
- [ ] Parse TCP headers in eBPF
- [ ] Extract memcache commands
- [ ] Generate Events in kernel
- [ ] Store in BPF maps
- [ ] Userspace reads pre-parsed events
- [ ] Benchmark: measure CPU savings

## References

- [aya documentation](https://docs.rs/aya/latest/aya/)
- [Linux TC (traffic control)](https://man7.org/linux/man-pages/man8/tc.8.html)
- [BPF & XDP](https://www.kernel.org/doc/html/latest/bpf/)
- [libbpf](https://github.com/libbpf/libbpf)

## Summary

**Current State:** PacketCapture abstraction allows adding eBPF support
- Trait-based design enables multiple backends
- Live capture and file replay work seamlessly
- API is stable for downstream (record/main.rs doesn't know about capture type)

**Next Steps to eBPF:**
1. Refactor into `PacketSource` trait (minimal effort, no eBPF code)
2. Implement `EbpfCapture` struct (medium effort)
3. Write eBPF programs in TC layer (high effort but optional)
4. Benchmark and measure real performance gains

**Recommendation:**
- Start with Phase 1 (trait abstraction) to decouple capture backends
- Phase 2-3 can be done incrementally
- Phase 4 (in-kernel parsing) only if benchmarks show significant gains
- Feature flag allows users to opt-in/out based on their platform
