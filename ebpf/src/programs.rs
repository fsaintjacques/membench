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
    // Read syscall arguments
    // arg 0: fd (socket file descriptor)
    // arg 1: buf (receive buffer pointer)
    // arg 2: len (buffer length)

    let fd: i32 = unsafe { ctx.read_at(16)? }; // fd is at offset 16
    let buf_ptr: u64 = unsafe { ctx.read_at(24)? }; // buf at offset 24
    let buf_len: usize = unsafe { ctx.read_at(32)? }; // len at offset 32

    // TODO: Get socket info (port) from fd
    // TODO: Filter for port 11211
    // TODO: Read data from buffer
    // TODO: Send to ringbuf

    info!(&ctx, "recv: fd={} buf_len={}", fd, buf_len);
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
