#![no_std]
#![allow(nonstandard_style, dead_code)]

// This eBPF program runs in kernel context.
// It filters packets at TC ingress for port 11211.
// Compiles to eBPF bytecode, not native machine code.
// Bytecode is platform-independent and portable across Linux distributions.

use aya_ebpf::{macros::*, helpers::*, bindings::*};
use aya_ebpf::maps::PerfEventArray;
use aya_log_ebpf::info;

#[map]
static PACKETS: PerfEventArray<u32> = PerfEventArray::new(0);

/// TC ingress hook for filtering memcache traffic
///
/// Filters packets by destination port (11211) and sends matching
/// packets to perf buffer for userspace processing.
///
/// This runs in kernel context with BPF verifier constraints.
#[tc]
pub fn filter_packets(ctx: TcContext) -> i32 {
    match try_filter_packets(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_filter_packets(ctx: TcContext) -> Result<i32, i32> {
    // TODO: Parse Ethernet header
    // TODO: Parse IP header and extract destination port
    // TODO: Check if port == 11211

    // For now, send all packets to perf buffer (for testing)
    // SAFETY: This is safe within eBPF context.
    // The verifier ensures no out-of-bounds access.
    unsafe {
        PACKETS.output(&ctx, &0, 0);
    }

    Ok(TC_ACT_OK)
}

// Action codes
const TC_ACT_OK: i32 = 0;
const TC_ACT_SHOT: i32 = 2;
