# membench Scripts

Utility scripts for operating membench.

## demo.sh - Full Workflow Demo

Orchestrates the complete membench workflow: capture → analyze → replay.

### What It Does

1. **Start memcached** - Launches a memcached daemon on a specified port
2. **Start membench record** - Begins capturing traffic from memcached
3. **Generate load** - Uses memtier_benchmark to create realistic traffic
4. **Stop membench record** - Completes traffic capture
5. **Replay profile** - Replays the captured traffic back against memcached

### Prerequisites

```bash
# Install required tools
brew install memcached memtier_benchmark  # macOS
# or
sudo apt-get install memcached memtier    # Ubuntu/Debian
```

### Setup Packet Capture Permissions (Avoid sudo)

**Linux**: Grant capabilities to the binary:
```bash
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/membench
getcap ./target/release/membench  # Verify
```

**macOS**: Grant permission to network interfaces:
```bash
sudo chmod +rw /dev/bpf*
```

After this setup, you can run `./scripts/demo.sh` without `sudo`.

### Usage

#### Basic Usage

```bash
./scripts/demo.sh
```

This runs with default settings:
- Memcached port: 11211
- Profile file: `/tmp/membench_demo.bin`
- Duration: 10 seconds
- Clients: 4
- Requests per client: 1000

#### With Custom Options

```bash
# Use a different port
./scripts/demo.sh --port 11212

# Save profile to a specific location
./scripts/demo.sh --output ~/my_profile.bin

# Keep profile after demo (normally it's deleted)
./scripts/demo.sh --keep-profile

# Run a shorter or longer load generation
./scripts/demo.sh --duration 20

# Customize memtier_benchmark workload
./scripts/demo.sh --clients 8 --requests 500

# Combine multiple options
./scripts/demo.sh \
  --port 11212 \
  --output /tmp/heavy_load.bin \
  --duration 30 \
  --clients 8 \
  --keep-profile
```

### Output Example

```
╔════════════════════════════════════════════════════════════════╗
║         membench Demo: Capture and Replay Workflow             ║
╚════════════════════════════════════════════════════════════════╝

Configuration:
  Memcached port: 11211
  Profile output: /tmp/membench_demo.bin
  memtier_benchmark: 4 clients, 1000 requests, 10s duration

[SUCCESS] All required tools found
[INFO] Building membench...
[SUCCESS] membench built successfully
[INFO] Starting memcached on port 11211...
[SUCCESS] memcached started (PID: 12345)
[INFO] Starting membench record mode...
[SUCCESS] membench record started (PID: 12346)
[INFO] Generating load with memtier_benchmark...
  Clients: 4
  Requests per client: 1000
  Duration: 10 seconds
[SUCCESS] Load generation complete
[INFO] Stopping membench record...
[SUCCESS] membench record stopped
[SUCCESS] Profile created: /tmp/membench_demo.bin (42K)
[INFO] Replaying profile...
  Profile: /tmp/membench_demo.bin
  Target: 127.0.0.1:11211
  (Replaying 1000 commands, press Ctrl+C to stop)
[SUCCESS] Replay complete

╔════════════════════════════════════════════════════════════════╗
║                    Demo Workflow Complete!                      ║
╚════════════════════════════════════════════════════════════════╝
```

### Important Notes

#### Network Packet Capture Requirements

The `membench record` command performs packet capture on your network interface, which requires elevated privileges on most systems.

**Depending on your system, you may need:**

```bash
# Option 1: Run with sudo
sudo ./scripts/demo.sh

# Option 2: Grant membench capture permissions (Linux)
# This is more secure than running the entire script with sudo
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/membench
./scripts/demo.sh
```

#### Loopback Interface

The script attempts to detect your system's loopback interface:
- **Linux**: `lo`
- **macOS**: `lo0`

If you get errors about interface not found, check:
```bash
# List your network interfaces
ifconfig
# or
ip link show
```

#### Port Already in Use

If port 11211 is already in use:

```bash
# Check what's using it
lsof -i :11211

# Kill existing memcached
pkill memcached

# Or use a different port
./scripts/demo.sh --port 11212
```

### Troubleshooting

#### "memcached not found"

Install memcached:
```bash
brew install memcached          # macOS
sudo apt-get install memcached  # Ubuntu
```

#### "memtier_benchmark not found"

Install memtier_benchmark:
```bash
brew install memtier_benchmark  # macOS
sudo apt-get install memtier    # Ubuntu
```

#### "Permission denied" during packet capture

Use sudo or grant capabilities:
```bash
sudo ./scripts/demo.sh
# or
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/membench
./scripts/demo.sh
```

#### "Interface not found"

The script couldn't find your loopback interface. Check available interfaces:
```bash
ifconfig | grep -A 1 "^lo"  # Linux
ifconfig | grep -A 1 "^lo0" # macOS
```

Then manually specify in the script or modify the interface detection logic.

#### Profile not created

If the profile file doesn't get created:
- Make sure you have permission to write to the output directory
- Check that membench record started successfully
- Verify the network interface name is correct
- Ensure you have packet capture privileges

### Environment

The script will:
1. Build membench in release mode if needed
2. Create memcached and membench processes in background
3. Automatically clean up processes on exit
4. Delete the profile file by default (use `--keep-profile` to retain it)

### Files Created/Modified

- Builds: `./target/release/membench`
- Profile: `/tmp/membench_demo.bin` (or custom location)
- Logs: None (output goes to stdout)

### Exit Codes

- `0` - Success
- `1` - Error (missing tools, build failed, etc.)
- Other - Signal from trap (INT, TERM)

### Examples

#### Quick Demo

```bash
./scripts/demo.sh
```

#### Long-running Benchmark

```bash
./scripts/demo.sh \
  --duration 60 \
  --clients 8 \
  --requests 5000 \
  --keep-profile
```

#### Multiple Iterations

```bash
for i in {1..3}; do
  echo "Run $i"
  ./scripts/demo.sh --keep-profile --output /tmp/run_$i.bin
done
```

#### Integration with Custom Analysis

```bash
# Run demo and keep profile
./scripts/demo.sh --keep-profile --output ./my_profile.bin

# Analyze the profile manually
./target/release/membench replay --input ./my_profile.bin --target 192.168.1.100:11211
```

### Extending the Script

To add more functionality:

1. **Custom workload patterns**: Modify `generate_load()` function
2. **Multiple replays**: Add loop after `replay_profile()`
3. **Metrics collection**: Capture output from memtier_benchmark and membench
4. **Profile comparison**: Compare multiple profiles created in sequence

### See Also

- [README.md](../README.md) - Main documentation
- [SYSTEM_TESTS.md](../docs/SYSTEM_TESTS.md) - System tests guide
- [USAGE.md](../docs/USAGE.md) - CLI usage guide
