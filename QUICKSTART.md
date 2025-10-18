# membench Quick Start

Get started with membench in 5 minutes.

## Installation

### Prerequisites

```bash
# Install memcached and memtier_benchmark
brew install memcached memtier_benchmark  # macOS
# or
sudo apt-get install memcached memtier    # Ubuntu
```

### Build membench

```bash
cargo build --release
./target/release/membench --help
```

### Grant Packet Capture Permissions (Optional)

To use `membench record` without `sudo`, grant the binary the necessary capabilities:

```bash
# Linux only - gives membench permission to capture packets
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/membench

# Verify it worked
getcap ./target/release/membench
# Output: ./target/release/membench = cap_net_admin,cap_net_raw+eip
```

**macOS**: Use `chmod +rw` on network interfaces instead:
```bash
sudo chmod +rw /dev/bpf*
```

After this, you can run `./scripts/demo.sh` without `sudo`.

## Quick Demo

Run the complete workflow in one command:

```bash
./scripts/demo.sh
```

This script:
1. Starts memcached
2. Begins capturing traffic with `membench record`
3. Generates load with `memtier_benchmark`
4. Stops the capture
5. Replays the captured profile with `membench replay`

**That's it!** Your first membench workflow is complete.

## Manual Workflow

If you prefer to run commands manually:

### Step 1: Start memcached

```bash
memcached -l 127.0.0.1 -p 11211 &
```

### Step 2: Start capturing traffic

In one terminal, start recording:

```bash
sudo ./target/release/membench record \
  --interface lo \
  --port 11211 \
  --output /tmp/capture.bin
```

(Replace `lo` with `lo0` on macOS if needed)

### Step 3: Generate load

In another terminal, generate traffic:

```bash
memtier_benchmark \
  --server 127.0.0.1 \
  --port 11211 \
  --protocol memcache_text \
  --clients 4 \
  --requests 1000 \
  --test-time 10
```

### Step 4: Stop recording

Stop the `membench record` process (Ctrl+C in the recording terminal).

### Step 5: Replay the profile

```bash
./target/release/membench replay \
  --input /tmp/capture.bin \
  --target 127.0.0.1:11211 \
  --concurrency 4
```

Press Ctrl+C to stop the replay.

## What You Get

membench captures and anonymizes:
- **Command types**: Get, Set, Delete, Noop
- **Key/value sizes**: Distribution of key and value sizes
- **Response patterns**: Hit/miss rates
- **Connection patterns**: Number of unique connections

It does **NOT** capture:
- Actual key or value contents (privacy!)
- Timing information between commands

## Next Steps

- **Read more**: See [README.md](./README.md) for full documentation
- **System tests**: Run `cargo test --test system_tests -- --ignored` to validate your setup
- **Unit tests**: Run `cargo test` to verify all functionality
- **Script options**: Run `./scripts/demo.sh --help` for advanced options

## Troubleshooting

### "permission denied" on packet capture

```bash
# Option 1: Use sudo
sudo ./scripts/demo.sh

# Option 2: Grant capabilities (more secure)
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/membench
./scripts/demo.sh
```

### "Address already in use"

```bash
# Kill existing memcached
pkill memcached

# Or use a different port
./scripts/demo.sh --port 11212
```

### "Interface not found"

Check your loopback interface:
```bash
# Linux
ifconfig lo

# macOS
ifconfig lo0
```

Then update the interface name in the command or script.

### Missing tools

Install requirements:
```bash
brew install memcached memtier_benchmark
# or
sudo apt-get install memcached memtier
```

## Key Features

✅ **Privacy-preserving** - Keys and values are hashed, never stored
✅ **Deterministic** - Same traffic pattern produces same profile with same salt
✅ **Lightweight** - Profiles are compact binary files
✅ **Realistic replay** - Traffic patterns match captured distributions
✅ **Simple CLI** - Two commands: `record` and `replay`

## Documentation

- **[README.md](./README.md)** - Full feature overview
- **[docs/USAGE.md](./docs/USAGE.md)** - Detailed CLI reference
- **[docs/SYSTEM_TESTS.md](./docs/SYSTEM_TESTS.md)** - System test guide
- **[scripts/README.md](./scripts/README.md)** - Demo script options

## Architecture

```
membench = Record Phase + Replay Phase

RECORD PHASE:
  Network packets → TCP reassembly → Protocol parsing → Key anonymization → Profile file

REPLAY PHASE:
  Profile file → Distribution analysis → Traffic generator → Memcached replay
```

## Performance

Typical throughput during replay:
- **Light**: ~1,000-2,000 Ops/sec
- **Moderate**: ~2,000-4,000 Ops/sec
- **Heavy**: ~4,000-8,000 Ops/sec

(Depends on hardware and system load)

## Getting Help

- Run `./target/release/membench --help` for CLI help
- Run `./scripts/demo.sh --help` for script options
- Check [SYSTEM_TESTS.md](./docs/SYSTEM_TESTS.md) for system test details

## What's Next?

1. ✅ Run the demo script
2. ✅ Capture your own workload
3. ✅ Analyze the profile
4. ✅ Replay against different targets
5. ✅ Use for benchmarking and load testing

Happy benchmarking! 🚀
