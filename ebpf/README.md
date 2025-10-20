# eBPF Socket Capture

This directory contains the eBPF kernel program for socket-level memcached traffic capture.

## Overview

The eBPF program intercepts `recv()` syscalls on port 11211, capturing already-reassembled TCP streams directly from the kernel. This eliminates packet fragmentation issues that plague traditional packet capture.

## Files

- `src/programs.rs` - eBPF kernel program source code
- `programs.bpf` - **Pre-compiled eBPF bytecode (committed)**
- `rebuild.sh` - Script to recompile bytecode from source
- `.cargo/config.toml` - Build configuration for eBPF target
- `Cargo.toml` - Dependencies for eBPF program

## Pre-compiled Bytecode

The `programs.bpf` file is **committed to the repository** to simplify builds. This means:

✅ **No special toolchain required** for normal builds
✅ **Faster builds** - No eBPF compilation
✅ **Works on all platforms** - Even macOS/Windows can build (though eBPF only runs on Linux)

## When to Rebuild

Rebuild the bytecode only when you modify `src/programs.rs`:

```bash
./rebuild.sh
```

### Requirements for Rebuilding

- Rust nightly toolchain with `rust-src` component
- `bpf-linker` tool
- Linux system (or WSL)

The `rebuild.sh` script will check for and install these automatically.

### After Rebuilding

1. **Test the changes:**
   ```bash
   cd ..
   cargo test --features ebpf
   ```

2. **Commit the updated bytecode:**
   ```bash
   git add programs.bpf
   git commit -m "ebpf: update bytecode for <your changes>"
   ```

## Architecture

### Kernel Side (eBPF)

**Tracepoint:** `syscalls:sys_enter_recvfrom`
- Runs when any process calls `recv()/recvfrom()`
- Filters for port 11211 (memcached)
- Reads data from userspace recv buffer
- Sends events to ringbuf

**Event Structure:**
```rust
struct SocketDataEvent {
    sock_id: u64,      // Socket FD
    sport: u16,        // Source port
    dport: u16,        // Destination port (11211)
    data_len: u32,     // Captured bytes
    data: [u8; 4096],  // Actual payload
}
```

### Userspace Side

The compiled bytecode is embedded into the main membench binary and loaded at runtime when using `ebpf:` capture mode.

## Why Socket-Level?

Traditional packet capture (libpcap) has fundamental limitations:

**Problem:** Memcache commands can span multiple TCP packets
```
Packet 1: "set longkey 0 0 100\r\nthis"
Packet 2: "istherestofthevalue\r\n"
```

**Packet capture:** Must reassemble TCP, complex and error-prone
**Socket capture:** Kernel already did reassembly, we get complete command

## Development

### Testing Changes

```bash
# 1. Modify src/programs.rs
# 2. Rebuild bytecode
./rebuild.sh

# 3. Build and test
cd ..
cargo build --features ebpf
cargo test --features ebpf

# 4. Integration test (requires root)
sudo ./target/release/membench record ebpf:any test.profile
```

### Debugging

The eBPF program includes `aya-log-ebpf` for kernel-side logging:

```rust
info!(&ctx, "captured {} bytes from memcached socket", copy_len);
```

To view logs, you'd need to set up `aya-log` in userspace (currently not configured).

## CI/CD

The committed bytecode approach makes CI simple - no special setup required. However, you can add a check to ensure bytecode stays current:

```yaml
- name: Verify eBPF bytecode is up-to-date
  run: |
    cd ebpf
    ./rebuild.sh
    git diff --exit-code programs.bpf || \
      (echo "eBPF bytecode is out of date. Run ./ebpf/rebuild.sh" && exit 1)
```

## Technical Details

- **Target:** `bpfel-unknown-none` (BPF little-endian, no std)
- **Build method:** `-Zbuild-std=core` (compile core from source)
- **Linker:** `bpf-linker` (specialized BPF linker)
- **Size:** ~131KB (includes debug info via `--btf` flag)

The bytecode is architecture-independent and works on x86_64, aarch64, etc.
