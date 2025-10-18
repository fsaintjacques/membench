#![no_std]
#![no_main]

use aya_ebpf::{
    macros::*,
    maps::RingBuf,
    programs::TracePointContext,
    helpers::bpf_probe_read_kernel,
};
use aya_log_ebpf::info;

// Maximum size for captured data per event
const MAX_DATA_SIZE: usize = 4096;

/// Event sent from kernel to userspace when socket recv occurs
#[repr(C)]
pub struct SocketDataEvent {
    /// Socket identifier (file descriptor)
    pub sock_id: u64,
    /// Source port
    pub sport: u16,
    /// Destination port
    pub dport: u16,
    /// Length of data
    pub data_len: u32,
    /// Actual data payload (up to MAX_DATA_SIZE)
    pub data: [u8; MAX_DATA_SIZE],
}

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// Tracepoint for sys_enter_recvfrom syscall
/// Captures data when applications recv() from sockets
#[tracepoint]
pub fn trace_recv_enter(ctx: TracePointContext) -> u32 {
    match try_trace_recv(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_trace_recv(ctx: TracePointContext) -> Result<u32, i64> {
    // TODO: Extract socket fd and port from context
    // TODO: Check if port == 11211
    // TODO: Capture recv data
    // TODO: Send to ringbuf

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
