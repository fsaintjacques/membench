# Replay Statistics Implementation Plan

> **For Claude:** Use `${SUPERPOWERS_SKILLS_ROOT}/skills/collaboration/executing-plans/SKILL.md` to implement this plan task-by-task.

**Goal:** Add comprehensive analytics to the replay facility with latency percentiles, throughput, error tracking, and per-operation type breakdowns, similar to memtier_benchmark.

**Architecture:** Per-connection statistics using HdrHistogram for latency tracking, periodic aggregation to a central stats collector task, with both live progress reporting and final summary output (text + optional JSON export). Request/response matching implemented to track full round-trip latency.

**Tech Stack:**
- `hdrhistogram` crate for percentile calculations
- `serde_json` for JSON export
- Existing tokio async architecture with MPSC channels

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add hdrhistogram and serde_json dependencies**

```bash
cargo add hdrhistogram
cargo add serde_json
```

Expected: Dependencies added to Cargo.toml

**Step 2: Verify build**

Run: `cargo build`
Expected: Build succeeds with new dependencies

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add hdrhistogram and serde_json dependencies for replay statistics"
```

---

## Task 2: Create ConnectionStats Structure

**Files:**
- Create: `src/replay/stats.rs`
- Modify: `src/replay/mod.rs` (add `pub mod stats;`)

**Step 1: Write the failing test**

Create `src/replay/stats.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::CommandType;
    use std::time::Duration;

    #[test]
    fn test_connection_stats_creation() {
        let stats = ConnectionStats::new(1);
        assert_eq!(stats.connection_id, 1);
    }

    #[test]
    fn test_record_latency() {
        let mut stats = ConnectionStats::new(1);
        stats.record_success(CommandType::Get, Duration::from_micros(100));
        stats.record_success(CommandType::Get, Duration::from_micros(200));

        assert_eq!(stats.get_count(), 2);
    }

    #[test]
    fn test_record_error() {
        let mut stats = ConnectionStats::new(1);
        stats.record_error(CommandType::Set, ErrorType::Timeout);

        assert_eq!(stats.get_error_count(), 1);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test stats::tests --lib`
Expected: FAIL with "ConnectionStats not found"

**Step 3: Write minimal implementation**

Add to `src/replay/stats.rs`:

```rust
use crate::profile::CommandType;
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    Timeout,
    ConnectionError,
    ProtocolError,
}

pub struct ConnectionStats {
    pub connection_id: u16,

    // Per-operation histograms (microsecond precision)
    histograms: HashMap<CommandType, Histogram<u64>>,

    // Success counters per operation
    success_counts: HashMap<CommandType, u64>,

    // Error tracking
    error_counts: HashMap<ErrorType, u64>,
}

impl ConnectionStats {
    pub fn new(connection_id: u16) -> Self {
        ConnectionStats {
            connection_id,
            histograms: HashMap::new(),
            success_counts: HashMap::new(),
            error_counts: HashMap::new(),
        }
    }

    pub fn record_success(&mut self, cmd_type: CommandType, latency: Duration) {
        let micros = latency.as_micros() as u64;

        // Update histogram
        let histogram = self.histograms
            .entry(cmd_type)
            .or_insert_with(|| Histogram::new(3).expect("Failed to create histogram"));
        histogram.record(micros).ok();

        // Update counter
        *self.success_counts.entry(cmd_type).or_insert(0) += 1;
    }

    pub fn record_error(&mut self, _cmd_type: CommandType, error_type: ErrorType) {
        *self.error_counts.entry(error_type).or_insert(0) += 1;
    }

    pub fn get_count(&self) -> u64 {
        self.success_counts.values().sum()
    }

    pub fn get_error_count(&self) -> u64 {
        self.error_counts.values().sum()
    }
}

#[cfg(test)]
mod tests {
    // ... tests from above
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test stats::tests --lib`
Expected: PASS (3 tests)

**Step 5: Update mod.rs**

Add to `src/replay/mod.rs`:

```rust
pub mod stats;
```

**Step 6: Run all tests**

Run: `cargo test`
Expected: All existing tests still pass

**Step 7: Commit**

```bash
git add src/replay/stats.rs src/replay/mod.rs
git commit -m "feat: add ConnectionStats structure for per-connection metrics"
```

---

## Task 3: Add StatsSnapshot for Aggregation

**Files:**
- Modify: `src/replay/stats.rs`

**Step 1: Write the failing test**

Add to `src/replay/stats.rs` tests:

```rust
#[test]
fn test_snapshot_creation() {
    let mut stats = ConnectionStats::new(1);
    stats.record_success(CommandType::Get, Duration::from_micros(100));
    stats.record_success(CommandType::Set, Duration::from_micros(200));

    let snapshot = stats.snapshot();
    assert_eq!(snapshot.connection_id, 1);
}

#[test]
fn test_snapshot_reset() {
    let mut stats = ConnectionStats::new(1);
    stats.record_success(CommandType::Get, Duration::from_micros(100));

    let _snapshot = stats.snapshot();
    assert_eq!(stats.get_count(), 0); // Should be reset after snapshot
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_snapshot --lib`
Expected: FAIL with "snapshot method not found"

**Step 3: Write minimal implementation**

Add to `src/replay/stats.rs`:

```rust
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub connection_id: u16,
    pub histograms: HashMap<CommandType, Histogram<u64>>,
    pub success_counts: HashMap<CommandType, u64>,
    pub error_counts: HashMap<ErrorType, u64>,
}

impl ConnectionStats {
    // ... existing methods ...

    /// Take a snapshot and reset counters (delta reporting)
    pub fn snapshot(&mut self) -> StatsSnapshot {
        let snapshot = StatsSnapshot {
            connection_id: self.connection_id,
            histograms: self.histograms.clone(),
            success_counts: self.success_counts.clone(),
            error_counts: self.error_counts.clone(),
        };

        // Reset for next interval
        self.histograms.clear();
        self.success_counts.clear();
        self.error_counts.clear();

        snapshot
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_snapshot --lib`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add src/replay/stats.rs
git commit -m "feat: add StatsSnapshot for delta reporting"
```

---

## Task 4: Create AggregatedStats

**Files:**
- Modify: `src/replay/stats.rs`

**Step 1: Write the failing test**

Add to `src/replay/stats.rs` tests:

```rust
#[test]
fn test_aggregated_stats_merge() {
    let mut agg = AggregatedStats::new();

    let mut stats1 = ConnectionStats::new(1);
    stats1.record_success(CommandType::Get, Duration::from_micros(100));

    let mut stats2 = ConnectionStats::new(2);
    stats2.record_success(CommandType::Get, Duration::from_micros(200));

    agg.merge(stats1.snapshot());
    agg.merge(stats2.snapshot());

    assert_eq!(agg.total_operations(), 2);
}

#[test]
fn test_aggregated_percentiles() {
    let mut agg = AggregatedStats::new();

    let mut stats = ConnectionStats::new(1);
    for i in 1..=100 {
        stats.record_success(CommandType::Get, Duration::from_micros(i * 10));
    }

    agg.merge(stats.snapshot());

    let p50 = agg.percentile(CommandType::Get, 50.0);
    assert!(p50.is_some());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_aggregated --lib`
Expected: FAIL with "AggregatedStats not found"

**Step 3: Write minimal implementation**

Add to `src/replay/stats.rs`:

```rust
pub struct AggregatedStats {
    // Merged histograms per operation type
    histograms: HashMap<CommandType, Histogram<u64>>,

    // Total counters
    success_counts: HashMap<CommandType, u64>,
    error_counts: HashMap<ErrorType, u64>,

    // Timing
    start_time: std::time::Instant,
}

impl AggregatedStats {
    pub fn new() -> Self {
        AggregatedStats {
            histograms: HashMap::new(),
            success_counts: HashMap::new(),
            error_counts: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn merge(&mut self, snapshot: StatsSnapshot) {
        // Merge histograms
        for (cmd_type, hist) in snapshot.histograms {
            let agg_hist = self.histograms
                .entry(cmd_type)
                .or_insert_with(|| Histogram::new(3).expect("Failed to create histogram"));
            agg_hist.add(&hist).ok();
        }

        // Merge success counts
        for (cmd_type, count) in snapshot.success_counts {
            *self.success_counts.entry(cmd_type).or_insert(0) += count;
        }

        // Merge error counts
        for (error_type, count) in snapshot.error_counts {
            *self.error_counts.entry(error_type).or_insert(0) += count;
        }
    }

    pub fn total_operations(&self) -> u64 {
        self.success_counts.values().sum()
    }

    pub fn percentile(&self, cmd_type: CommandType, percentile: f64) -> Option<u64> {
        self.histograms
            .get(&cmd_type)
            .map(|h| h.value_at_percentile(percentile))
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn throughput(&self) -> f64 {
        let elapsed = self.elapsed_secs();
        if elapsed > 0.0 {
            self.total_operations() as f64 / elapsed
        } else {
            0.0
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_aggregated --lib`
Expected: PASS (2 tests)

**Step 5: Commit**

```bash
git add src/replay/stats.rs
git commit -m "feat: add AggregatedStats for merging connection statistics"
```

---

## Task 5: Add JSON Export Support

**Files:**
- Modify: `src/replay/stats.rs`

**Step 1: Write the failing test**

Add to `src/replay/stats.rs` tests:

```rust
#[test]
fn test_json_export() {
    let mut agg = AggregatedStats::new();

    let mut stats = ConnectionStats::new(1);
    stats.record_success(CommandType::Get, Duration::from_micros(100));
    stats.record_success(CommandType::Set, Duration::from_micros(200));

    agg.merge(stats.snapshot());

    let json = agg.to_json().expect("Failed to serialize");
    assert!(json.contains("\"Get\""));
    assert!(json.contains("\"Set\""));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_json_export --lib`
Expected: FAIL with "to_json method not found"

**Step 3: Write minimal implementation**

Add to top of `src/replay/stats.rs`:

```rust
use serde::{Serialize, Deserialize};
use serde_json;
```

Add derive macros and implementation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    // ... existing variants
}

#[derive(Debug, Serialize)]
pub struct JsonStats {
    pub elapsed_secs: f64,
    pub total_operations: u64,
    pub throughput: f64,
    pub operations: HashMap<String, OperationStats>,
    pub errors: HashMap<String, u64>,
}

#[derive(Debug, Serialize)]
pub struct OperationStats {
    pub count: u64,
    pub p50_micros: u64,
    pub p95_micros: u64,
    pub p99_micros: u64,
    pub min_micros: u64,
    pub max_micros: u64,
}

impl AggregatedStats {
    // ... existing methods ...

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let mut operations = HashMap::new();

        for (cmd_type, hist) in &self.histograms {
            let count = self.success_counts.get(cmd_type).copied().unwrap_or(0);
            let op_stats = OperationStats {
                count,
                p50_micros: hist.value_at_percentile(50.0),
                p95_micros: hist.value_at_percentile(95.0),
                p99_micros: hist.value_at_percentile(99.0),
                min_micros: hist.min(),
                max_micros: hist.max(),
            };
            operations.insert(format!("{:?}", cmd_type), op_stats);
        }

        let mut errors = HashMap::new();
        for (error_type, count) in &self.error_counts {
            errors.insert(format!("{:?}", error_type), *count);
        }

        let json_stats = JsonStats {
            elapsed_secs: self.elapsed_secs(),
            total_operations: self.total_operations(),
            throughput: self.throughput(),
            operations,
            errors,
        };

        serde_json::to_string_pretty(&json_stats)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_json_export --lib`
Expected: PASS

**Step 5: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/replay/stats.rs
git commit -m "feat: add JSON export support for aggregated statistics"
```

---

## Task 6: Modify ConnectionTask to Track Latency

**Files:**
- Modify: `src/replay/connection_task.rs`
- Create: `tests/connection_stats_tests.rs`

**Step 1: Write the failing test**

Create `tests/connection_stats_tests.rs`:

```rust
use membench::replay::stats::ConnectionStats;
use membench::profile::{CommandType, Event, Flags};
use std::time::Duration;

#[test]
fn test_connection_stats_tracking() {
    let mut stats = ConnectionStats::new(1);

    // Simulate tracking a request
    let start = std::time::Instant::now();
    std::thread::sleep(Duration::from_micros(100));
    let latency = start.elapsed();

    stats.record_success(CommandType::Get, latency);

    assert_eq!(stats.get_count(), 1);
    assert!(stats.get_count() > 0);
}
```

**Step 2: Run test to verify it passes (baseline)**

Run: `cargo test connection_stats_tracking`
Expected: PASS (this validates our stats module works)

**Step 3: Modify connection_task.rs to accept stats channel**

Update `src/replay/connection_task.rs`:

```rust
use super::client::ReplayClient;
use super::stats::ConnectionStats;
use super::ProtocolMode;
use crate::profile::Event;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use std::time::Instant;

pub async fn spawn_connection_task(
    target: &str,
    rx: mpsc::Receiver<Event>,
    stats_tx: mpsc::Sender<ConnectionStats>,
    connection_id: u16,
    protocol_mode: ProtocolMode,
) -> Result<tokio::task::JoinHandle<Result<()>>> {
    let target = target.to_string();

    let handle = tokio::spawn(async move {
        let mut client = ReplayClient::new(&target, protocol_mode).await?;
        let mut rx = rx;
        let mut local_stats = ConnectionStats::new(connection_id);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    let start = Instant::now();

                    if let Err(e) = client.send_command(&event).await {
                        local_stats.record_error(event.cmd_type, super::stats::ErrorType::ConnectionError);
                        return Err(e);
                    }

                    if let Err(e) = client.read_response().await {
                        local_stats.record_error(event.cmd_type, super::stats::ErrorType::ProtocolError);
                        return Err(e);
                    }

                    let latency = start.elapsed();
                    local_stats.record_success(event.cmd_type, latency);
                }
                _ = interval.tick() => {
                    // Send snapshot every 2 seconds
                    let snapshot = local_stats.snapshot();
                    if stats_tx.send(snapshot).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                else => break,
            }
        }

        Ok(())
    });

    Ok(handle)
}
```

**Step 4: Update function signature exports**

Verify the module exports are correct in `src/replay/mod.rs`

**Step 5: Build to check compilation**

Run: `cargo build`
Expected: FAIL - main.rs needs updating to match new signature

**Step 6: Commit intermediate work**

```bash
git add src/replay/connection_task.rs tests/connection_stats_tests.rs
git commit -m "feat: add latency tracking to connection task"
```

---

## Task 7: Create StatsAggregator Task

**Files:**
- Create: `src/replay/stats_aggregator.rs`
- Modify: `src/replay/mod.rs`

**Step 1: Write the implementation**

Create `src/replay/stats_aggregator.rs`:

```rust
use super::stats::{AggregatedStats, StatsSnapshot};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub async fn spawn_stats_aggregator(
    mut rx: mpsc::Receiver<StatsSnapshot>,
    should_exit: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<AggregatedStats> {
    tokio::spawn(async move {
        let mut agg_stats = AggregatedStats::new();
        let mut report_interval = tokio::time::interval(std::time::Duration::from_secs(5));

        loop {
            tokio::select! {
                Some(snapshot) = rx.recv() => {
                    agg_stats.merge(snapshot);
                }
                _ = report_interval.tick() => {
                    // Live progress report
                    let elapsed = agg_stats.elapsed_secs();
                    let total_ops = agg_stats.total_operations();
                    let throughput = agg_stats.throughput();

                    tracing::info!(
                        "[{:.0}s] Operations: {} | Throughput: {:.0} ops/sec",
                        elapsed,
                        total_ops,
                        throughput
                    );
                }
                else => {
                    if should_exit.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }

        agg_stats
    })
}
```

**Step 2: Update mod.rs**

Add to `src/replay/mod.rs`:

```rust
mod stats_aggregator;
pub use stats_aggregator::spawn_stats_aggregator;
```

**Step 3: Build**

Run: `cargo build`
Expected: Build succeeds

**Step 4: Commit**

```bash
git add src/replay/stats_aggregator.rs src/replay/mod.rs
git commit -m "feat: add stats aggregator task for live reporting"
```

---

## Task 8: Integrate Stats into Replay Main

**Files:**
- Modify: `src/replay/main.rs`

**Step 1: Update imports**

Add to `src/replay/main.rs`:

```rust
use crate::replay::{spawn_stats_aggregator, stats::StatsSnapshot};
```

**Step 2: Modify main replay loop**

Update the `run` function in `src/replay/main.rs`:

Remove the old `sent_counter` and `_reporting_handle` logic (lines 54, 84-115, 126-127).

Add after Phase 1 (after line 49):

```rust
    // Phase 1.5: Create stats aggregator
    let (stats_tx, stats_rx) = mpsc::channel::<StatsSnapshot>(1000);
    let stats_handle = spawn_stats_aggregator(stats_rx, Arc::clone(&should_exit)).await;
```

Update Phase 2 to pass stats channel (replace lines 52-65):

```rust
    // Phase 2: Create SPSC queues for each connection
    let mut connection_queues: HashMap<u16, mpsc::Sender<Event>> = HashMap::new();
    let mut connection_tasks = Vec::new();

    for &conn_id in &unique_connections {
        let (tx, rx) = mpsc::channel(1000);
        connection_queues.insert(conn_id, tx);

        let target = target.to_string();
        let stats_tx_clone = stats_tx.clone();

        let task_handle = spawn_connection_task(
            &target,
            rx,
            stats_tx_clone,
            conn_id,
            protocol_mode
        ).await?;
        connection_tasks.push(task_handle);
    }

    // Drop our copy of stats_tx so aggregator can finish
    drop(stats_tx);
```

Update Phase 6 (after line 122) to get final stats:

```rust
    // Phase 6: Wait for all connection tasks to drain queues and finish
    for task in connection_tasks {
        task.await??;
    }

    // Phase 7: Signal stats aggregator and get final results
    should_exit.store(true, Ordering::Relaxed);
    let final_stats = stats_handle.await?;

    // Final summary
    print_final_summary(&final_stats);
```

**Step 3: Add summary printing function**

Add at end of `src/replay/main.rs`:

```rust
use crate::profile::CommandType;

fn print_final_summary(stats: &crate::replay::stats::AggregatedStats) {
    tracing::info!("=== Replay Complete ===");
    tracing::info!("Elapsed: {:.2}s", stats.elapsed_secs());
    tracing::info!("Total Operations: {}", stats.total_operations());
    tracing::info!("Throughput: {:.2} ops/sec", stats.throughput());

    for cmd_type in [CommandType::Get, CommandType::Set, CommandType::Delete, CommandType::Noop] {
        if let Some(p50) = stats.percentile(cmd_type, 50.0) {
            let p95 = stats.percentile(cmd_type, 95.0).unwrap_or(0);
            let p99 = stats.percentile(cmd_type, 99.0).unwrap_or(0);

            tracing::info!(
                "{:?} latency (μs) - p50: {}, p95: {}, p99: {}",
                cmd_type,
                p50,
                p95,
                p99
            );
        }
    }
}
```

**Step 4: Build**

Run: `cargo build`
Expected: Build succeeds (may need to fix import paths)

**Step 5: Fix any compilation errors**

Adjust imports as needed based on compiler feedback

**Step 6: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 7: Commit**

```bash
git add src/replay/main.rs
git commit -m "feat: integrate stats tracking into replay main loop"
```

---

## Task 9: Add CLI Flag for JSON Export

**Files:**
- Modify: `src/main.rs` (replay subcommand args)
- Modify: `src/replay/main.rs`

**Step 1: Add JSON output path argument**

Find the replay subcommand definition in `src/main.rs` and add:

```rust
#[arg(long, value_name = "FILE")]
stats_json: Option<String>,
```

**Step 2: Pass to replay::main::run**

Update the call to `replay::main::run` in `src/main.rs`:

```rust
replay::main::run(
    &file,
    &target,
    &loop_mode,
    protocol_mode,
    should_exit,
    stats_json.as_deref(),
)
.await
```

**Step 3: Update run signature**

Update `src/replay/main.rs` run function signature:

```rust
pub async fn run(
    input: &str,
    target: &str,
    loop_mode: &str,
    protocol_mode: ProtocolMode,
    should_exit: Arc<AtomicBool>,
    stats_json: Option<&str>,
) -> Result<()> {
```

**Step 4: Add JSON export after final summary**

Add after `print_final_summary` in `src/replay/main.rs`:

```rust
    // Export JSON if requested
    if let Some(json_path) = stats_json {
        let json = final_stats.to_json()?;
        std::fs::write(json_path, json)?;
        tracing::info!("Statistics exported to {}", json_path);
    }
```

**Step 5: Build and test**

Run: `cargo build`
Expected: Build succeeds

**Step 6: Test help text**

Run: `cargo run -- replay --help`
Expected: Shows --stats-json flag

**Step 7: Commit**

```bash
git add src/main.rs src/replay/main.rs
git commit -m "feat: add --stats-json CLI flag for exporting replay statistics"
```

---

## Task 10: Integration Testing

**Files:**
- Create: `tests/replay_stats_integration_test.rs`

**Step 1: Write integration test**

Create `tests/replay_stats_integration_test.rs`:

```rust
use membench::profile::{CommandType, Event, Flags};
use membench::replay::stats::{ConnectionStats, AggregatedStats};
use std::time::Duration;
use std::num::NonZero;

#[test]
fn test_stats_workflow() {
    // Simulate connection task workflow
    let mut conn1 = ConnectionStats::new(1);
    let mut conn2 = ConnectionStats::new(2);

    // Simulate events
    for i in 1..=50 {
        conn1.record_success(CommandType::Get, Duration::from_micros(i * 10));
        conn2.record_success(CommandType::Set, Duration::from_micros(i * 20));
    }

    // Take snapshots
    let snap1 = conn1.snapshot();
    let snap2 = conn2.snapshot();

    // Aggregate
    let mut agg = AggregatedStats::new();
    agg.merge(snap1);
    agg.merge(snap2);

    // Verify
    assert_eq!(agg.total_operations(), 100);
    assert!(agg.percentile(CommandType::Get, 50.0).is_some());
    assert!(agg.percentile(CommandType::Set, 50.0).is_some());

    // JSON export
    let json = agg.to_json().expect("Failed to export JSON");
    assert!(json.contains("Get"));
    assert!(json.contains("Set"));
}

#[test]
fn test_stats_reset_after_snapshot() {
    let mut stats = ConnectionStats::new(1);
    stats.record_success(CommandType::Get, Duration::from_micros(100));

    let snapshot = stats.snapshot();
    assert_eq!(snapshot.success_counts.get(&CommandType::Get), Some(&1));

    // After snapshot, stats should be reset
    assert_eq!(stats.get_count(), 0);
}
```

**Step 2: Run integration tests**

Run: `cargo test replay_stats_integration`
Expected: PASS (2 tests)

**Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add tests/replay_stats_integration_test.rs
git commit -m "test: add integration tests for replay statistics workflow"
```

---

## Task 11: Documentation

**Files:**
- Modify: `README.md`

**Step 1: Update README with statistics section**

Add after the "Replay Mode" section in `README.md`:

```markdown
### Replay Statistics

The replay command provides comprehensive performance analytics similar to memtier_benchmark:

- **Latency Percentiles**: p50, p95, p99 for each operation type (Get, Set, Delete)
- **Throughput**: Operations per second
- **Error Tracking**: Timeouts, connection errors, protocol errors
- **Per-Operation Breakdown**: Separate statistics for each command type

#### Live Progress

During replay, statistics are reported every 5 seconds:

```bash
[5s] Operations: 15000 | Throughput: 3000 ops/sec
[10s] Operations: 32000 | Throughput: 3200 ops/sec
```

#### Final Summary

When replay completes, a detailed summary is printed:

```bash
=== Replay Complete ===
Elapsed: 60.23s
Total Operations: 180000
Throughput: 2989.21 ops/sec
Get latency (μs) - p50: 245, p95: 512, p99: 1024
Set latency (μs) - p50: 198, p95: 445, p99: 892
```

#### JSON Export

Export detailed statistics to JSON for further analysis:

```bash
membench replay profile.bin --stats-json stats.json
```

Example JSON output:

```json
{
  "elapsed_secs": 60.23,
  "total_operations": 180000,
  "throughput": 2989.21,
  "operations": {
    "Get": {
      "count": 120000,
      "p50_micros": 245,
      "p95_micros": 512,
      "p99_micros": 1024,
      "min_micros": 89,
      "max_micros": 2456
    },
    "Set": {
      "count": 60000,
      "p50_micros": 198,
      "p95_micros": 445,
      "p99_micros": 892,
      "min_micros": 76,
      "max_micros": 1823
    }
  },
  "errors": {}
}
```
```

**Step 2: Verify README renders correctly**

Run: `cat README.md | grep -A 20 "### Replay Statistics"`
Expected: Shows the new section

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add replay statistics documentation to README"
```

---

## Task 12: Manual End-to-End Test

**Files:**
- None (manual testing)

**Step 1: Build release binary**

Run: `cargo build --release`
Expected: Build succeeds

**Step 2: Verify replay command help**

Run: `./target/release/membench replay --help`
Expected: Shows --stats-json option

**Step 3: Create a test profile (if available)**

If you have a sample profile file, test with:

```bash
./target/release/membench replay sample.profile --loop-mode once --stats-json /tmp/stats.json
```

Expected:
- Live progress reports every 5s
- Final summary printed
- JSON file created at /tmp/stats.json

**Step 4: Verify JSON output**

Run: `cat /tmp/stats.json | head -20`
Expected: Valid JSON with operations and percentiles

**Step 5: Document results**

If manual test passes, note any observations. If no test profile available, skip to next task.

---

## Task 13: Final Verification and Cleanup

**Files:**
- All modified files

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings or errors

**Step 3: Format code**

Run: `cargo fmt`
Expected: Code formatted

**Step 4: Build in release mode**

Run: `cargo build --release`
Expected: Build succeeds

**Step 5: Review all changes**

Run: `git diff main`
Expected: Review all changes make sense

**Step 6: Final commit if needed**

```bash
git add -A
git commit -m "chore: final cleanup and formatting"
```

---

## Summary

This plan implements comprehensive replay statistics with:

1. **ConnectionStats**: Per-connection latency tracking with HdrHistogram
2. **StatsAggregator**: Central collection and live reporting
3. **Request/Response Matching**: Full round-trip latency measurement
4. **Multi-format Output**: Live progress + final text summary + optional JSON export
5. **Per-operation Breakdown**: Separate stats for Get/Set/Delete/Noop

The implementation follows TDD principles with tests at each step, maintains existing functionality, and integrates cleanly with the current async architecture.
