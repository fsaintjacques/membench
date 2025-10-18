# Phase 1: Capture Trait Abstraction Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/executing-plans/SKILL.md` to implement this plan task-by-task.

**Goal:** Refactor the capture layer to use a `PacketSource` trait, enabling unlimited future backends (eBPF, XDP, AF_XDP) without modifying downstream code.

**Architecture:**
- Extract `PacketSource` trait defining the capture interface (`next_packet()`, `source_info()`, `is_finite()`)
- Create concrete implementations: `LiveCapture`, `FileCapture` wrapping pcap
- Refactor `PacketCapture` to use `Box<dyn PacketSource>` instead of enum
- Update factory method to dispatch to correct implementation
- No changes needed downstream (record/main.rs works unchanged)

**Tech Stack:** Rust trait objects, pcap crate, no new dependencies

**Effort:** ~3 hours, 10 tasks

---

## Task 1: Create PacketSource Trait Definition

**Files:**
- Modify: `src/record/capture.rs` (lines 1-10, add before enum)
- Test: Trait compiles, no runtime test needed

**Step 1: Add trait definition to top of capture.rs**

After the imports in `src/record/capture.rs`, add:

```rust
/// Common interface for packet capture backends
pub trait PacketSource {
    /// Read next packet from source
    fn next_packet(&mut self) -> Result<&[u8]>;

    /// Get human-readable source description (interface name or file path)
    fn source_info(&self) -> &str;

    /// Whether source is finite (file) vs continuous (interface)
    fn is_finite(&self) -> bool;

    /// Optional: Get capture statistics (when available)
    fn stats(&self) -> Option<CaptureStats> {
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
```

**Step 2: Verify trait compiles**

```bash
cargo check
```

Expected: No errors, trait definition accepted.

**Step 3: Commit**

```bash
git add src/record/capture.rs
git commit -m "feat: add PacketSource trait definition

Define common interface for capture backends:
- next_packet(): Read next packet
- source_info(): Get source description
- is_finite(): Whether source ends
- stats(): Optional statistics

Enables: Live, Offline, eBPF, XDP backends"
```

---

## Task 2: Extract LiveCapture Implementation

**Files:**
- Modify: `src/record/capture.rs` (replace existing Live logic)
- Test: Compiles (no functional test yet)

**Step 1: Add LiveCapture struct**

After the trait definition, add:

```rust
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

    fn stats(&self) -> Option<CaptureStats> {
        self.handle.stats().ok().map(|s| CaptureStats {
            packets_received: s.received,
            packets_dropped: s.dropped,
            bytes_received: 0,
        })
    }
}
```

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: No errors.

**Step 3: Commit**

```bash
git add src/record/capture.rs
git commit -m "feat: extract LiveCapture struct implementing PacketSource

Move live interface capture logic into dedicated struct.
Implements PacketSource trait for network interfaces."
```

---

## Task 3: Extract FileCapture Implementation

**Files:**
- Modify: `src/record/capture.rs` (add after LiveCapture)
- Test: Compiles

**Step 1: Add FileCapture struct**

After LiveCapture impl, add:

```rust
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
```

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: No errors.

**Step 3: Commit**

```bash
git add src/record/capture.rs
git commit -m "feat: extract FileCapture struct implementing PacketSource

Move PCAP file capture logic into dedicated struct.
Implements PacketSource trait for offline files."
```

---

## Task 4: Remove Old CaptureHandle Enum

**Files:**
- Modify: `src/record/capture.rs` (remove enum definition)
- Test: Compiles

**Step 1: Delete the CaptureHandle enum**

Remove this from the file:

```rust
enum CaptureHandle {
    Live(Capture<pcap::Active>),
    Offline(Capture<pcap::Offline>),
}
```

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: Error - PacketCapture still references CaptureHandle. That's next.

**Step 3: Don't commit yet - move to next task**

---

## Task 5: Refactor PacketCapture to Use Trait Objects

**Files:**
- Modify: `src/record/capture.rs` (PacketCapture struct)
- Test: Compiles

**Step 1: Replace PacketCapture struct**

Replace the old struct:

```rust
pub struct PacketCapture {
    source: Box<dyn PacketSource>,
}

impl PacketCapture {
    pub fn is_file(source: &str) -> bool {
        std::path::Path::new(source).is_file()
    }

    pub fn list_devices() -> Result<Vec<String>> {
        let devices = pcap::Device::list()
            .context("failed to list devices")?;
        Ok(devices.into_iter().map(|d| d.name).collect())
    }

    pub fn next_packet(&mut self) -> Result<&[u8]> {
        self.source.next_packet()
    }

    pub fn source_info(&self) -> &str {
        self.source.source_info()
    }

    pub fn is_finite(&self) -> bool {
        self.source.is_finite()
    }

    pub fn stats(&self) -> Option<CaptureStats> {
        self.source.stats()
    }
}
```

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: No errors yet - we haven't implemented `from_source()` yet.

**Step 3: Don't commit yet - move to next task**

---

## Task 6: Implement from_source Factory Method

**Files:**
- Modify: `src/record/capture.rs` (add to impl PacketCapture)
- Test: Compiles

**Step 1: Add from_source factory method**

Add to `impl PacketCapture` block:

```rust
    /// Create a packet capture from a source (interface or PCAP file)
    /// Auto-detects the type by checking if source is a file
    pub fn from_source(source: &str, port: u16) -> Result<Self> {
        let packet_source: Box<dyn PacketSource> = if Self::is_file(source) {
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
```

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: No errors.

**Step 3: Don't commit yet - move to next task**

---

## Task 7: Remove from_source Old Implementation

**Files:**
- Modify: `src/record/capture.rs` (remove old from_source code)
- Test: Compiles

**Step 1: Remove old implementation**

The old `from_source` method that had the enum matching logic - delete it. Keep only the new one from Task 6.

**Step 2: Verify compiles**

```bash
cargo check
```

Expected: No errors.

**Step 3: Commit all refactoring changes**

```bash
git add src/record/capture.rs
git commit -m "refactor: replace CaptureHandle enum with PacketSource trait

Replaces enum-based dispatch with trait object pattern:
- Removed CaptureHandle enum
- Updated PacketCapture to use Box<dyn PacketSource>
- Implemented from_source factory with auto-detection
- Kept new() for backwards compatibility
- No functional changes, architecture improvement only

All existing code works unchanged (capture.new() still works)."
```

---

## Task 8: Update record/main.rs to Use is_finite()

**Files:**
- Modify: `src/record/main.rs` (lines 19-30, logging section)
- Test: Compiles and runs

**Step 1: Update source type logging**

Replace this section:

```rust
    let source_type = if PacketCapture::is_file(source) { "file" } else { "interface" };
    tracing::info!("Recording from {} ({}):{} to {}", source_type, source, port, output);
    tracing::debug!("Salt: {}", salt);
    tracing::info!("Capturing memcache traffic... Press Ctrl+C to stop.");

    if !PacketCapture::is_file(source) {
        tracing::debug!("Available devices: {:?}", PacketCapture::list_devices().unwrap_or_default());
    }

    // Initialize components
    let mut capture = PacketCapture::from_source(source, port)?;
    tracing::debug!("Capture initialized from {}: {}", source_type, source);
```

With:

```rust
    let mut capture = PacketCapture::from_source(source, port)?;
    let source_type = if capture.is_finite() { "file" } else { "interface" };
    tracing::info!("Recording from {} ({}):{} to {}", source_type, source, port, output);
    tracing::debug!("Salt: {}", salt);
    tracing::info!("Capturing memcache traffic... Press Ctrl+C to stop.");

    if !capture.is_finite() {
        tracing::debug!("Available devices: {:?}", PacketCapture::list_devices().unwrap_or_default());
    }

    tracing::debug!("Capture initialized from {}: {}", source_type, capture.source_info());
```

**Step 2: Verify compiles and runs**

```bash
cargo build --release
```

Expected: No errors.

**Step 3: Commit**

```bash
git add src/record/main.rs
git commit -m "refactor: use capture.is_finite() instead of file detection

Updates logging to use trait method instead of static file check.
More accurate for future backends."
```

---

## Task 9: Run Full Test Suite

**Files:**
- Test: `cargo test --all`

**Step 1: Run all tests**

```bash
cargo test --all
```

Expected: All 26 tests pass, 0 warnings, 0 failures.

**Step 2: Verify no warnings**

```bash
cargo build --release 2>&1 | grep -E "warning|error"
```

Expected: No output (no warnings or errors).

**Step 3: Run individual record test**

```bash
cargo test --test record_capture_tests
```

Expected: PASS (if that test exists, or similar name).

**Step 4: Document results**

If all pass, note the commit hash:

```bash
git log --oneline -1
```

---

## Task 10: Final Verification and Summary

**Files:**
- Verify: All files build clean
- Document: Changes summary

**Step 1: Clean build**

```bash
cargo clean && cargo build --release
```

Expected: Builds in ~2 seconds, no errors or warnings.

**Step 2: Verify structure**

Check the file structure is correct:

```bash
grep -n "pub trait PacketSource" src/record/capture.rs
grep -n "pub struct LiveCapture" src/record/capture.rs
grep -n "pub struct FileCapture" src/record/capture.rs
grep -n "pub struct PacketCapture" src/record/capture.rs
```

Expected: All 4 definitions found.

**Step 3: View final file**

```bash
wc -l src/record/capture.rs
```

Expected: Should be slightly longer (new trait + stats struct) but cleaner organization.

**Step 4: Create summary commit (optional)**

```bash
git log --oneline -10 | head -10
```

Verify the 6-8 commits from this phase are present.

**Step 5: Document completion**

Phase 1 is complete! The trait abstraction is now in place. The codebase is ready for:
- ✅ Phase 2: eBPF implementation (add `EbpfCapture` struct, no downstream changes)
- ✅ Phase 3: In-kernel parsing (enhance eBPF, still no downstream changes)
- ✅ Future backends: XDP, AF_XDP, etc. (just add new trait impl)

---

## Success Criteria

- ✅ All 26 tests pass
- ✅ Zero compiler warnings
- ✅ `cargo build --release` succeeds
- ✅ No changes needed in `record/main.rs` functionality (only logging improved)
- ✅ `PacketCapture::new()` still works (backwards compatible)
- ✅ Trait object abstraction in place and working
- ✅ Ready for Phase 2 (eBPF) implementation

## Notes for Implementation

1. **No functional changes**: This phase only reorganizes code, no new features
2. **Fully backwards compatible**: Existing code paths unchanged
3. **Trait object overhead**: ~1-2% on hot path (negligible)
4. **Future-proof**: Adding new backends requires only creating new trait impl
5. **Testable**: Each task produces a clean, compilable state
