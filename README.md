# membench

A privacy-preserving memcache traffic capture and replay tool for benchmarking realistic workloads without exposing sensitive keys or values.

## Overview

membench lets you capture memcache traffic patterns from production environments and replay them for benchmarking, performance testing, and capacity planning—all while maintaining privacy by anonymizing keys and values.

The tool works in two phases:

- **Record**: Passively captures memcache traffic via libpcap, anonymizes keys using deterministic hashing, and stores only size distributions and command patterns in a compact binary profile
- **Replay**: Reads a profile and generates semi-deterministic traffic matching the captured command, key size, and value size distributions against a target memcached server

## Key Features

- **Privacy-Preserving**: Keys and values are never stored. Only command types, size distributions, and response patterns are captured
- **Deterministic Hashing**: Keys are hashed with a configurable salt, allowing reproducible anonymization across runs
- **Zero Production Impact**: Uses passive network capture (libpcap) with no instrumentation or modifications to memcached servers
- **Compact Profiles**: Binary serialization keeps captured profiles small, even for large traffic volumes
- **Realistic Replay**: Generates traffic matching captured distributions, maintaining realistic hit rates, command mixes, and key/value sizes
- **Async I/O**: Replay engine uses async I/O for efficient concurrent traffic generation

## Installation

### From Source

```bash
git clone https://github.com/yourusername/membench.git
cd membench
cargo install --path .
```

### Requirements

- Rust 1.70+
- libpcap development headers (`libpcap-dev` on Debian/Ubuntu, `libpcap` on macOS)
- Network interface access (typically requires `sudo` for capture mode)

## Quick Start

### 1. Capture Traffic from Production

```bash
# Capture from eth0 on port 11211
sudo membench record \
  --interface eth0 \
  --port 11211 \
  --output production.profile \
  --salt 12345
```

This will run indefinitely, capturing all memcache traffic on eth0. Press Ctrl+C to stop.

The `--salt` parameter makes key anonymization reproducible. Omit it to use a random salt.

### 2. Analyze the Profile

```bash
# View profile statistics
membench profile --input production.profile
```

Output shows command distribution, hit rate, key/value size ranges, and connection patterns.

### 3. Replay Against a Test Environment

```bash
# Replay with 4 concurrent connections (Ctrl+C to stop)
membench replay \
  --input production.profile \
  --target test-memcached:11211 \
  --concurrency 4
```

Monitor the target memcached server during replay to observe performance metrics. Press Ctrl+C to stop the replay.

## Usage Guide

### Record Mode

Captures memcache traffic from a live network interface.

```bash
membench record [OPTIONS] --interface <INTERFACE> --output <OUTPUT>
```

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--interface` | *required* | Network interface to capture from (e.g., `lo`, `eth0`, `en0`) |
| `--port` | `11211` | Memcache port to filter on |
| `--output` | *required* | Path to write the profile binary file |
| `--salt` | *random* | Salt for deterministic key hashing (for reproducible anonymization) |

#### Examples

```bash
# Capture from localhost (requires sudo)
sudo membench record --interface lo --port 11211 --output local.profile

# Capture from production network interface with fixed salt
sudo membench record \
  --interface eth0 \
  --port 11211 \
  --output production.profile \
  --salt 0x1234567890abcdef

# Capture non-standard memcache port
sudo membench record --interface eth1 --port 11212 --output custom_port.profile
```

### Replay Mode

Replays captured traffic patterns against a target memcached server.

```bash
membench replay [OPTIONS] --input <INPUT> --target <TARGET>
```

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--input` | *required* | Path to the profile binary file |
| `--target` | `localhost:11211` | Target memcached address |
| `--concurrency` | `4` | Number of concurrent connections |

#### Examples

```bash
# Replay against localhost
membench replay --input production.profile

# High-concurrency test against production-like environment
membench replay \
  --input production.profile \
  --target memcache-cluster:11211 \
  --concurrency 32

# Smoke test with 2 connections
membench replay --input test.profile --concurrency 2
```

### Profile Inspection

View statistics and metadata from a profile without replaying.

```bash
membench profile --input <PROFILE>
```

Shows:
- Total events captured
- Unique connections
- Command distribution (Get/Set/Delete/Noop percentages)
- Key size distribution
- Value size distribution
- Cache hit rate
- Time range of capture

## Profile Format

Profiles are binary files created by the record mode. They contain:

1. **ProfileMetadata** (bincode serialized)
   - Magic number (0xDEADBEEF)
   - Version (1)
   - Total event count
   - Time range (first/last timestamp)
   - Unique connection count
   - Command distribution histogram

2. **Event Stream** (bincode serialized)
   - Timestamp (u64)
   - Connection ID (u32)
   - Command type (Get/Set/Delete/Noop)
   - Key hash (u64)
   - Key size (u32)
   - Value size (Option<u32>)
   - Flags (bitfield)
   - Response type (Found/NotFound/Error)

The format is compact and self-describing, allowing profiles to be exchanged between different versions of membench (with backward compatibility guarantees).

## How It Works

### Recording

1. libpcap captures TCP packets on the specified interface/port
2. TCP streams are reassembled from individual packets
3. Memcache binary protocol is parsed from stream data
4. Keys are anonymized using SipHash with a configurable salt
5. Events (command type, key hash, key size, value size, response) are serialized and written to the profile file

### Replaying

1. Profile file is read and deserialized
2. Event distributions are analyzed (command mix, key sizes, value sizes, hit rate)
3. Traffic generator samples from distributions to create realistic command sequences
4. Commands are sent asynchronously to the target memcached server using the specified concurrency level
5. Statistics are collected and reported

## Privacy Guarantees

- **No Key Storage**: Original keys are never stored. Only hashes are recorded.
- **No Value Storage**: Values themselves are never stored. Only sizes are recorded.
- **Deterministic Hashing**: Same key always produces same hash within a profile (same salt)
- **Anonymous Replay**: Replayed commands use synthetic keys that match the captured size/distribution but don't correspond to original keys

Even with access to a profile file, it's computationally infeasible to recover original keys (they would need to be brute-forced through the hash function).

## Performance Characteristics

- **Capture Overhead**: Minimal—uses passive network capture with no packet modification
- **Profile Size**: Typically 100-500 bytes per event (varies with metadata overhead)
- **Replay Throughput**: 5,000-50,000 commands/sec depending on target memcached and concurrency level
- **Memory Usage**: Profiles are fully loaded into memory; a 1GB profile contains ~2-10 million events

## Limitations & Future Work

- Currently supports binary memcache protocol only (not ASCII protocol)
- Pipelining and multi-get/multi-set commands are recorded as individual events
- Replay does not match inter-command timing from the capture (purely request rate based)
- No support for SASL authentication or TLS
- Connection pooling follows a simple round-robin pattern during replay

## Contributing

Contributions are welcome. Please open issues for bugs or feature requests.

## License

[Your License Here]

## See Also

- [Memcache Binary Protocol](https://github.com/memcached/memcached/blob/master/doc/protocol-binary.txt)
- [libpcap Documentation](https://www.tcpdump.org/papers/sniffing-faq.html)
