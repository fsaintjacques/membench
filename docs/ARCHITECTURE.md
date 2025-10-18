# Membench Architecture & Internal Structure

## Overview

Membench is a privacy-preserving memcache traffic capture and replay tool. It operates in three phases:
1. **Record**: Capture memcache traffic from network interface or PCAP file
2. **Analyze**: Extract distribution metrics from recorded traffic
3. **Replay**: Replay captured traffic against a target memcache server

## Module Hierarchy

```
membench/
├── lib.rs                          # Library exports
├── main.rs                         # CLI entry point
│
├── profile/                        # Shared data model
│   └── mod.rs                      # Event, CommandType, Flags, ProfileMetadata
│
├── record/                         # Traffic capture & anonymization
│   ├── mod.rs                      # Module exports
│   ├── main.rs                     # Record command orchestration
│   ├── capture.rs                  # PacketCapture (Live + Offline)
│   ├── parser.rs                   # MemcacheParser (protocol parsing)
│   ├── anonymizer.rs               # Anonymizer (key hashing)
│   └── writer.rs                   # ProfileWriter (binary format)
│
├── replay/                         # Traffic replay engine
│   ├── mod.rs                      # Module exports, ProtocolMode
│   ├── main.rs                     # Replay orchestration
│   ├── client.rs                   # ReplayClient (async TCP)
│   ├── reader.rs                   # ProfileReader (binary deserializer)
│   ├── streamer.rs                 # ProfileStreamer (event streaming)
│   ├── connection_task.rs          # Per-connection async task
│   ├── reader_task.rs              # Event distribution task
│   └── analyzer.rs                 # Distribution analysis
│
└── analyze/                        # Traffic analysis
    ├── mod.rs                      # Module exports
    └── main.rs                     # Analyze command
```

## Data Flow

### Record Phase
```
Network Interface / PCAP File
         ↓
   PacketCapture (pcap crate)
         ↓
   [Optional] TCP Stream Extraction
         ↓
   MemcacheParser
    (extract cmd, key, value_size)
         ↓
   Anonymizer.hash_key()
         ↓
   Event (32 bytes, structured)
         ↓
   ProfileWriter (bincode + length-prefix)
         ↓
   Profile File (binary format v2)
```

### Replay Phase
```
Profile File
      ↓
ProfileReader
      ↓
ProfileStreamer (deserialize events)
      ↓
reader_task (distribute to connections)
      ↓
SPSC Channels (per conn_id)
      ↓
connection_task (spawn per unique connection)
      ↓
ReplayClient (TCP + ProtocolMode)
      ↓
Memcache Server
```

### Analyze Phase
```
Profile File
      ↓
ProfileReader
      ↓
DistributionAnalyzer
      ↓
Distribution Metrics
  (command distribution, size distributions, etc.)
```

## Key Data Structures

### Event (32 bytes, optimized layout)
```rust
pub struct Event {
    timestamp: u64,                      // 8 bytes - temporal marker
    conn_id: u16,                        // 2 bytes - connection identity (u32→u16)
    cmd_type: CommandType,               // 1 byte  - Get/Set/Delete/Noop
    flags: Flags,                        // 1 byte  - quiet, has_value bits
    key_hash: u64,                       // 8 bytes - anonymized key
    key_size: u32,                       // 4 bytes - key length
    value_size: Option<NonZero<u32>>,   // 4 bytes - niche-optimized (None→0 implicitly)
}
```

**Optimizations:**
- `conn_id: u32 → u16` (saves 2 bytes, supports 65k connections)
- `value_size: Option<u32> → Option<NonZero<u32>>` (niche optimization, no space overhead)
- Field reordering: temporal, then identity, then cmd metadata, then key/value info
- Result: 20% size reduction (40→32 bytes) vs earlier versions

### ProfileMetadata
```rust
pub struct ProfileMetadata {
    magic: u32,                              // 0xDEADBEEF
    version: u8,                             // v2 (current)
    total_events: u64,
    time_range: (u64, u64),                 // (first_ts, last_ts)
    unique_connections: u32,
    command_distribution: HashMap<CommandType, u64>,
}
```

### Profile File Format (Binary, Length-Prefixed)
```
[Global Header: PCAP-style metadata]
[Event₁ Length: u16] [Event₁ Data: bincode serialized]
[Event₂ Length: u16] [Event₂ Data: bincode serialized]
...
[EventN Length: u16] [EventN Data: bincode serialized]
[Metadata Length: u16] [Metadata Data: bincode serialized]
[End Marker: 0xDEADBEEF u32]
```

**Why length-prefixed?**
- Enables streaming deserialization
- Allows skipping malformed events
- Supports partial file reads

## Component Details

### 1. Capture Layer (record/capture.rs)

**CaptureHandle Enum**
```rust
enum CaptureHandle {
    Live(Capture<pcap::Active>),    // Real-time network interface
    Offline(Capture<pcap::Offline>), // PCAP file replay
}
```

**Auto-detection Logic**
```rust
pub fn from_source(source: &str, port: u16) -> Result<Self> {
    if Path::new(source).is_file() {
        // Open as PCAP file
        Capture::from_file(source)?
    } else {
        // Open as live interface
        Capture::from_device(source)?.open()?
    }
}
```

**Key Features:**
- Unified API for both modes (same `next_packet()`)
- BPF filter applied to both: `tcp port 11211`
- Backwards compatible: `new()` delegates to `from_source()`

### 2. Protocol Parsing (record/parser.rs)

**MemcacheParser**
- Parses both ASCII and Meta protocols
- Extracts: command type, key range, value size
- Handles: GET, SET, DELETE, NOOP (and mg, ms, md, mn variants)
- Returns: ParsedCommand with byte ranges (no allocation for keys)

```rust
pub fn parse_command(&self, input: &[u8]) -> Result<(ParsedCommand, &[u8])> {
    // Lazy evaluation: key_range is Range<usize>, not String
}
```

### 3. Anonymization (record/anonymizer.rs)

**Anonymizer**
- Uses SipHash-2-4 with random salt
- Deterministic: same key always hashes to same value
- Salt-based: prevents rainbow tables across captures
- Zero-allocation: consumes &[u8] key bytes directly

```rust
pub fn hash_key(&self, key: &[u8]) -> u64 {
    // Returns u64 hash, stores in Event.key_hash
}
```

### 4. Binary Serialization (record/writer.rs)

**ProfileWriter**
- Uses `bincode` for efficient serialization
- Tracks metadata while writing:
  - Total events
  - Time range (first/last timestamp)
  - Unique connections
  - Command distribution
- Buffers writes (BufWriter)

**Key Design Decision:**
```rust
// Write event with length prefix for streaming
self.file.write_all(&(encoded.len() as u16).to_le_bytes())?;
self.file.write_all(&encoded)?;
```

### 5. Replay Architecture (replay/)

**Three-Task Model:**

**Task 1: Reader Task** (reader_task.rs)
```rust
pub async fn reader_task(
    input: &str,
    connection_queues: HashMap<u16, mpsc::Sender<Event>>,
    loop_mode: LoopMode,
    should_exit: Arc<AtomicBool>,
) -> Result<()>
```
- Reads profile file sequentially
- Routes events to per-connection SPSC channels
- Supports looping: once, infinite, times:N
- Respects should_exit signal

**Task 2: Connection Tasks** (connection_task.rs)
```rust
pub async fn spawn_connection_task(
    target: &str,
    rx: mpsc::Receiver<Event>,
    sent_counter: Arc<AtomicU64>,
    protocol_mode: ProtocolMode,
) -> Result<tokio::task::JoinHandle<Result<()>>>
```
- Spawned once per unique connection ID
- Receives events from SPSC channel
- Maintains single TCP connection to target
- Generates commands based on protocol mode
- Drains TCP buffer (critical for throughput)

**Task 3: Reporting Task** (main.rs)
```rust
// Logs throughput every 5 seconds
let throughput = sent as f64 / elapsed;
tracing::info!("[{:.0}s] Sent: {} | Throughput: {:.0} ops/sec", ...)
```

### 6. Command Generation (replay/client.rs)

**ReplayClient**
```rust
pub struct ReplayClient {
    stream: TcpStream,           // Async TCP
    buffer: Vec<u8>,             // Read buffer (65KB)
    protocol_mode: ProtocolMode, // Ascii or Meta
}
```

**Protocol Support:**
- **ASCII**: `get key\r\n`, `set key 0 0 size\r\nvalue\r\n`, `delete key\r\n`
- **Meta**: `mg key v\r\n`, `ms key size\r\nvalue\r\n`, `md key\r\n`

**Deterministic Key/Value Generation:**
```rust
fn generate_key(&self, key_hash: u64, key_size: u32) -> String {
    // Convert hash to hex, repeat/truncate to key_size
    // Same hash+size → same key
}

fn generate_value(&self, size: u32) -> String {
    // Fill with pattern (e.g., "xxxxx...")
}
```

### 7. Distribution Analysis (replay/analyzer.rs)

**DistributionAnalyzer**
```rust
pub struct AnalysisResult {
    pub total_events: u64,
    pub command_distribution: HashMap<CommandType, u64>,
    pub key_size_distribution: HashMap<u32, u64>,
    pub value_size_distribution: HashMap<u32, u64>,
    pub inter_arrival_times: Vec<u64>,
}
```

**Computes:**
- Command type frequencies (Get/Set/Delete ratios)
- Key size distribution (histogram)
- Value size distribution (histogram)
- Inter-arrival times (latency patterns)

## CLI Interface

### Record
```bash
membench record <SOURCE> <OUTPUT> [--port PORT] [--salt SALT] [-v|-vv|-vvv]
```
- `SOURCE`: Interface name (eth0) or PCAP file (traffic.pcap)
- `OUTPUT`: Profile file destination (binary format v2)
- `--port`: Filter port (default 11211)
- `--salt`: Anonymization salt (default: current timestamp)

### Analyze
```bash
membench analyze <FILE> [-v|-vv|-vvv]
```
- `FILE`: Profile file to analyze

### Replay
```bash
membench replay <FILE> [--target TARGET] [--loop-mode MODE] [--protocol-mode MODE] [-v|-vv|-vvv]
```
- `FILE`: Profile file to replay
- `--target`: Server address (default: localhost:11211)
- `--loop-mode`: once, infinite, times:N (default: once)
- `--protocol-mode`: ascii or meta (default: meta)

## Concurrency Model

**Record Phase:** Sequential
- Single-threaded event capture and parsing
- Serialization to disk

**Replay Phase:** Async Multi-Connection
```
┌─────────────────┐
│  Reader Task    │ - Reads profile file sequentially
│  (main tokio)   │ - Routes to per-connection queues
└────────┬────────┘
         │
    ┌────┴────┬────────────┐
    ↓         ↓            ↓
┌─────────┐┌─────────┐ ┌─────────┐
│ Conn 1  ││ Conn 2  │ │ Conn N  │ - Each connection has:
│ Task    ││ Task    │ │ Task    │   - SPSC channel (reader→task)
│ (send)  ││ (send)  │ │ (send)  │   - TCP connection
└─────────┘└─────────┘ └─────────┘   - Event-to-command converter
```

**Advantages:**
- Per-connection socket affinity (realistic workload)
- True concurrency (not thread pools with overhead)
- Backpressure: SPSC channel buffers (1000 events per connection)
- Easy to monitor: arc<AtomicU64> counter

## Testing Strategy

### Unit Tests
- Parser tests (protocol edge cases)
- Analyzer tests (distribution computation)
- Client tests (command generation)

### Integration Tests
- Round-trip serialization (Event → binary → Event)
- Large profile handling (1000+ events)
- PCAP file detection and opening

### System Tests (Optional, require external tools)
- Live capture with tcpdump/memtier
- End-to-end workflow validation

## Performance Characteristics

### Memory Usage
- Event struct: 32 bytes (optimized)
- 1.6M events: ~51 MB in memory (Event vector)
- Profile file: ~51 MB on disk (bincode is space-efficient)
- Replay: O(unique_connections) memory (SPSC channels)

### Throughput
- Capture: ~tens of thousands of packets/second (pcap limited)
- Replay: 80k+ ops/sec (limited by target memcache capacity)
- Serialize/Deserialize: negligible vs I/O

### Latency Patterns
- TCP buffer draining: critical for sustained throughput
- Connection reuse: per-connection affinity preserved
- Reader task: sequential (preserves temporal order per connection)

## Extension Points for eBPF

### Current State
- **Capture:** pcap crate abstraction (Live + Offline)
- **Parser:** In-userspace (robust error handling)
- **Anonymization:** Userspace SipHash
- **Serialization:** Bincode format

### Where eBPF Could Fit
1. **Kernel-space capture** (would replace current `Live` path)
   - Lower overhead than pcap
   - Direct BPF maps for aggregation
   - Challenge: Must preserve Event struct compatibility

2. **In-kernel filtering**
   - Filter by payload content (not just port)
   - Aggregate small events in kernel
   - Challenge: Kernel code complexity

3. **Packet to Event conversion** (kernel-side)
   - Parse TCP stream reconstruction
   - Generate Event structs in kernel
   - Challenge: Memory management, safety

### Integration Pattern
```rust
enum CaptureHandle {
    Live(Capture<pcap::Active>),        // Current
    Ebpf(EbpfCapture),                  // New
    Offline(Capture<pcap::Offline>),    // Current
}

// Could share same next_packet() interface
impl EbpfCapture {
    pub fn next_packet(&mut self) -> Result<&[u8]> {
        // Read from BPF maps/perf buffer
    }
}
```

### eBPF Implementation Strategy
1. Start with userspace capture (current)
2. Add eBPF bypass path for kernel capture
3. Preserve Event format and downstream analysis
4. Benchmark kernel vs userspace
5. Optionally add in-kernel aggregation

## Design Philosophy

1. **Separation of Concerns**
   - Record, Analyze, Replay are independent commands
   - Clean interface between phases
   - Easy to extend (e.g., add new protocols)

2. **Data Privacy**
   - Keys anonymized via salted hash
   - Values not stored (only sizes)
   - Metadata is aggregated (no individual flows)

3. **Reproducibility**
   - Binary format enables exact replay
   - Deterministic key/value generation
   - Connection topology preserved

4. **Extensibility**
   - ProtocolMode enum for protocol variants
   - CaptureHandle for capture backends
   - LoopMode for replay variations

5. **Zero-Copy Where Possible**
   - Event references (not allocations) in parser
   - Binary serialization (not text)
   - SPSC channels for thread-safe queues
