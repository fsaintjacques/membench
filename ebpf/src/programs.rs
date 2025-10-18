#![no_std]
#![no_main]

use aya_ebpf::{
    macros::*,
    maps::RingBuf,
    programs::TracePointContext,
    helpers::{bpf_probe_read_kernel, bpf_probe_read_user},
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

/// Read from userspace buffer safely
unsafe fn bpf_probe_read_user_buf(src: *const u8, dst: &mut [u8]) -> Result<(), i64> {
    let len = dst.len();
    bpf_probe_read_user(dst.as_mut_ptr() as *mut _, len as u32, src as *const _)
        .map_err(|e| e as i64)
}

/// Get destination port from socket fd
/// Returns Ok(port) if this is a TCP socket, Err otherwise
fn get_socket_port(fd: i32) -> Result<u16, i64> {
    // For now, we'll use a simplified approach:
    // Check if this is a memcached connection by port
    // In production, would use bpf_get_socket_cookie or similar

    // TODO: Implement actual socket port lookup
    // For MVP, we'll filter in userspace
    Ok(11211) // Placeholder - return target port
}

fn try_trace_recv(ctx: TracePointContext) -> Result<u32, i64> {
    let fd: i32 = unsafe { ctx.read_at(16)? };
    let buf_ptr: u64 = unsafe { ctx.read_at(24)? };
    let buf_len: usize = unsafe { ctx.read_at(32)? };

    let port = get_socket_port(fd)?;
    if port != 11211 {
        return Ok(0);
    }

    // Limit data size to prevent exceeding stack limits
    let copy_len = if buf_len > MAX_DATA_SIZE {
        MAX_DATA_SIZE
    } else {
        buf_len
    };

    // Reserve space in ringbuf
    let mut event = EVENTS.reserve::<SocketDataEvent>(0).ok_or(-1i64)?;

    // Populate event
    event.sock_id = fd as u64;
    event.sport = 0; // TODO: Extract from socket
    event.dport = port;
    event.data_len = copy_len as u32;

    // Read data from userspace buffer
    unsafe {
        bpf_probe_read_user_buf(
            buf_ptr as *const u8,
            &mut event.data[..copy_len],
        )?;
    }

    // Submit to ringbuf
    event.submit(0);

    info!(&ctx, "captured {} bytes from memcached socket", copy_len);
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
