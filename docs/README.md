# Membench Documentation Index

Welcome to the membench documentation. This directory contains comprehensive guides for understanding, using, and extending the membench codebase.

## Quick Navigation

### 👶 I'm New to Membench
Start here:
1. **[MEMBENCH_CODEBASE_OVERVIEW.md](MEMBENCH_CODEBASE_OVERVIEW.md)** - High-level introduction (30 mins read)
   - What membench is and why it exists
   - Three-phase architecture overview
   - Quick walkthrough of data flow

2. **[ARCHITECTURE.md](ARCHITECTURE.md)** - Deep dive into design (1-2 hours read)
   - Complete module hierarchy
   - Detailed component descriptions
   - Data structures and binary formats
   - Performance characteristics

### 🔧 I Want to Extend Membench
Pick what you need:
- **Adding a new protocol?** → See "Extension Guide" in MEMBENCH_CODEBASE_OVERVIEW.md
- **Adding a new capture backend?** → See "Extension Points" in ARCHITECTURE.md
- **Implementing eBPF support?** → Read EBPF_INTEGRATION.md + CAPTURE_TRAIT_DESIGN.md

### 🚀 I'm Interested in eBPF
Read in order:
1. **[CAPTURE_TRAIT_DESIGN.md](CAPTURE_TRAIT_DESIGN.md)** - Trait abstraction pattern (1 hour)
   - Why traits enable extensibility
   - Concrete implementation patterns
   - Testing strategy
   - Performance implications

2. **[EBPF_INTEGRATION.md](EBPF_INTEGRATION.md)** - eBPF roadmap (1-2 hours)
   - Why eBPF for packet capture
   - Three-stage implementation approach
   - Kernel requirements and dependencies
   - 4-week implementation plan
   - References and resources

### 📊 I Want Statistics
**[ANALYSIS_SUMMARY.md](ANALYSIS_SUMMARY.md)** - Session summary
- Key findings from code analysis
- Code statistics and metrics
- Feasibility assessment for eBPF
- Next steps and recommendations

### 🧪 I'm Running Tests
**[SYSTEM_TESTS.md](SYSTEM_TESTS.md)** - Test documentation
- Unit tests
- Integration tests
- System tests (requires external tools)
- Running tests locally

## Document Overview

| Document | Size | Focus | Best For |
|----------|------|-------|----------|
| [MEMBENCH_CODEBASE_OVERVIEW.md](MEMBENCH_CODEBASE_OVERVIEW.md) | 393 lines | High-level architecture | Getting started, explaining to others |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 469 lines | Deep design details | Understanding internals, implementing features |
| [EBPF_INTEGRATION.md](EBPF_INTEGRATION.md) | 441 lines | eBPF roadmap | Planning eBPF implementation |
| [CAPTURE_TRAIT_DESIGN.md](CAPTURE_TRAIT_DESIGN.md) | 508 lines | Trait patterns | Before implementing trait refactoring |
| [ANALYSIS_SUMMARY.md](ANALYSIS_SUMMARY.md) | 315 lines | Session summary | Quick reference of findings |

**Total:** 2,126 lines of documentation

## Key Concepts

### Three-Phase Architecture
```
RECORD → ANALYZE → REPLAY
```
Each phase is independent and can be used separately.

### Core Data Model
- **Event**: 32-byte struct with timestamp, connection ID, command type, key/value info
- **ProfileMetadata**: Aggregated statistics (command distribution, time range, etc.)
- **Binary Format**: Length-prefixed events with metadata

### Extensibility Points
1. **Capture Backends**: Live interface, PCAP file, eBPF (planned), AF_XDP (future)
2. **Protocols**: Memcache ASCII, Memcache Meta, Redis (future), others
3. **Analysis**: Additional metrics and visualizations
4. **Replay Modes**: Loop modes, protocol variants, performance optimizations

## Architecture at a Glance

### Modules
- `profile/`: Event struct and data model
- `record/`: Capture, parse, anonymize, serialize
- `replay/`: Read, route, replay against target server
- `analyze/`: Compute statistics from profile
- `lib.rs`: Library exports
- `main.rs`: CLI interface

### Key Files
- `src/record/capture.rs`: PacketCapture abstraction (Live + Offline)
- `src/record/parser.rs`: Memcache protocol parser
- `src/replay/client.rs`: Async TCP client with protocol generation
- `src/replay/main.rs`: Three-task async orchestration

## Common Tasks

### Running Membench

Capture traffic:
```bash
membench record eth0 capture.bin
```

Analyze profile:
```bash
membench analyze capture.bin
```

Replay against server:
```bash
membench replay capture.bin --target localhost:11211
```

### Running Tests
```bash
# All tests
cargo test --all

# Specific test file
cargo test --test pcap_file_tests

# With output
cargo test -- --nocapture
```

### Building Release
```bash
cargo build --release
# Binary: target/release/membench
```

## Adding a New Feature

### Adding a Protocol
See "Adding a New Protocol" in MEMBENCH_CODEBASE_OVERVIEW.md

### Adding a Capture Backend
1. Implement the capture logic
2. Create factory method in `PacketCapture::from_source()`
3. Add tests
4. Update documentation

### Adding Analysis Metrics
1. Extend `AnalysisResult` struct
2. Compute metric in `DistributionAnalyzer::analyze()`
3. Output in `analyze/main.rs`
4. Add test

## Performance Targets

- **Capture**: 50k-100k packets/sec (libpcap limited)
- **Replay**: 80k-100k ops/sec (target server dependent)
- **Memory**: 32 bytes per event (optimized)
- **Event file**: ~51 MB for 1.6M events

## Contributing

1. Read relevant documentation section
2. Follow existing code patterns
3. Ensure tests pass: `cargo test --all`
4. No compiler warnings: `cargo build --release`
5. Add/update documentation

## FAQ

**Q: Can I use membench in production?**
A: Yes, the code is production-ready. Run with proper access controls (record requires network interface access, replay needs network connectivity).

**Q: Can I capture on non-loopback interfaces?**
A: Yes, membench works on any interface (eth0, enp0s31f6, etc.). May require elevated privileges (sudo).

**Q: Does membench capture response data?**
A: No, only command types and sizes. This is intentional for privacy. Response payloads are not stored.

**Q: How can I extend membench?**
A: See the "Extension Guide" in MEMBENCH_CODEBASE_OVERVIEW.md or the specific feature in ARCHITECTURE.md.

**Q: Is eBPF support planned?**
A: Yes, see EBPF_INTEGRATION.md for detailed roadmap. Would add 2-3x performance improvement.

**Q: Can I use membench on macOS/Windows?**
A: Yes, record and replay work on any OS with libpcap (macOS, Windows). eBPF is Linux-only but with graceful fallback.

## Resources

### External References
- [libpcap documentation](https://www.tcpdump.org/papers/sniffing-faq.html)
- [Tokio async runtime](https://tokio.rs/)
- [Bincode serialization](https://docs.rs/bincode/)
- [Memcache protocol](https://github.com/memcached/memcached/blob/master/doc/protocol.txt)

### Tools
- `tcpdump`: Create PCAP files for offline analysis
- `memtier_benchmark`: Generate memcache load for testing
- `memcached`: Target server for replay testing
- `wireshark`: Inspect PCAP files graphically

## Getting Help

### Debugging Tips
- Use `-vvv` flag for verbose output
- Check logs with `RUST_LOG=debug`
- Run tests: `cargo test --all`
- Build without optimization: `cargo build`

### Common Issues
- **"Permission denied" on record**: Run with `sudo`
- **"No such device"**: Wrong interface name, use `membench record --help`
- **"Connection refused" on replay**: Memcached not running on target

## Document Maintenance

This documentation was generated in a comprehensive code analysis session and represents the membench codebase as of the time indicated in commit history.

To keep docs up-to-date:
1. Update when making architectural changes
2. Add notes about new features
3. Reflect performance changes if benchmarking
4. Link to new modules/files as they're added

---

**Last Updated:** October 18, 2024
**Membench Version:** Current main branch
**Status:** ✅ Production-Ready

For questions or improvements to this documentation, refer to ANALYSIS_SUMMARY.md for session notes or review the git history for context.
