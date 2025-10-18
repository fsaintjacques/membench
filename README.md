# membench

A memcache traffic capture and replay tool for benchmarking realistic workloads without exposing sensitive keys or values.

## Overview

membench lets you capture memcache traffic patterns from production environments and replay them for benchmarking, performance testing, and capacity planning—all while maintaining privacy by anonymizing keys and values.

The tool works in two phases:

- **Record**: Passively captures memcache traffic via libpcap, anonymizes keys using deterministic hashing, and stores command patterns and size distributions in a compact binary profile
- **Replay**: Reads a profile and replays the exact captured events with preserved connection topology, deterministic key/value generation, and configurable looping modes against a target memcached server

## Key Features

- **Deterministic Hashing**: Keys are hashed with a configurable salt, allowing reproducible anonymization across runs
- **Zero Production Impact**: Uses passive network capture (libpcap) with no instrumentation or modifications to memcached servers
- **Reproducible Replay**: Replays the sequence of commands from capture, preserving connection topology and access patterns
- **Deterministic Keys/Values**: Generates reproducible distribution of keys and values based on captured hashes and sizes

## Quick Start

### 1. Capture Traffic from Production

```bash
# Capture from eth0 on port 11211
sudo membench record eth0 production.profile --port 11211
```

This will run indefinitely, capturing all memcache traffic on eth0. Press Ctrl+C to stop.

### 2. Analyze the Profile

```bash
# View profile statistics
membench analyze production.profile
```

Output shows command distribution, hit rate, key/value size ranges, and connection patterns.

### 3. Replay Against a Test Environment

```bash
# Replay once against a test server
membench replay production.profile --target test-memcached:11211

# Replay infinitely (Ctrl+C to stop)
membench replay production.profile --target test-memcached:11211 --loop-mode infinite

# Replay 3 times
membench replay production.profile --target test-memcached:11211 --loop-mode times:3
```

Monitor the target memcached server during replay to observe performance metrics. Press Ctrl+C to stop the replay.

## Usage Guide

### Record Mode

Captures memcache traffic from a live network interface.

```bash
membench record [OPTIONS] <INTERFACE> <OUTPUT>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `<INTERFACE>` | Network interface to capture from (e.g., `lo`, `eth0`, `en0`) |
| `<OUTPUT>` | Path to write the profile binary file |

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--port` | `11211` | Memcache port to filter on |
| `--salt` | *random* | Salt for deterministic key hashing (for reproducible anonymization) |

#### Examples

```bash
# Capture from localhost (requires sudo)
sudo membench record lo local.profile --port 11211

# Capture from production network interface with fixed salt
sudo membench record eth0 production.profile --port 11211 --salt 0x1234567890abcdef

# Capture non-standard memcache port
sudo membench record eth1 custom_port.profile --port 11212
```

### Replay Mode

Replays captured traffic patterns against a target memcached server with support for different looping modes.

```bash
membench replay [OPTIONS] <FILE>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `<FILE>` | Path to the profile binary file |

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--target` | `localhost:11211` | Target memcached address |
| `--loop-mode` | `once` | Loop mode: `once`, `infinite`, or `times:N` (e.g., `times:3`) |

#### Examples

```bash
# Replay once against localhost
membench replay production.profile

# Replay infinitely against production-like environment (Ctrl+C to stop)
membench replay production.profile --target memcache-cluster:11211 --loop-mode infinite

# Replay 10 times
membench replay production.profile --loop-mode times:10

# Smoke test with specific target
membench replay test.profile --target 192.168.1.10:11211
```

### Profile Inspection

View statistics and metadata from a profile without replaying.

```bash
membench analyze <FILE>
```

Shows:
- Total events captured
- Unique connections
- Command distribution (Get/Set/Delete/Noop percentages)
- Key size distribution
- Value size distribution
- Cache hit rate
- Time range of capture

## How It Works

### Recording

1. libpcap captures TCP packets on the specified interface/port
2. TCP streams are reassembled from individual packets
3. Memcache binary protocol is parsed from stream data
4. Keys are anonymized using SipHash with a configurable salt
5. Events (command type, key hash, key size, value size, response) are serialized and written to the profile file

### Replaying

1. Profile file is streamed and deserialized event-by-event
2. A reader task coordinates event distribution to per-connection async tasks
3. Connection tasks are spawned based on unique connection IDs from the capture
4. Events are replayed in their original order, preserving connection topology
5. Keys and values are deterministically generated from captured hashes and sizes
6. Commands are sent asynchronously to the target memcached server
7. Statistics are collected and reported
8. Looping repeats the profile based on configured mode (once, N times, or infinite)

## Privacy with distribution preserving

- **No Key Storage**: Original keys are never stored. Only hashes are recorded.
- **Deterministic Hashing**: Same key always produces same hash within a profile (same salt)
- **Anonymous Replay**: Replayed commands use synthetic keys that match the captured size/distribution but don't correspond to original keys

Even with access to a profile file, it's impractical to recover original keys if using random salt (the default)

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

## See Also

- [Memcache Binary Protocol](https://github.com/memcached/memcached/blob/master/doc/protocol-binary.txt)
- [libpcap Documentation](https://www.tcpdump.org/papers/sniffing-faq.html)