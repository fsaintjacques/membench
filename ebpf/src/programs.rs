#![no_std]
#![no_main]

use aya_ebpf::{
    macros::*,
    maps::{Array, HashMap, RingBuf},
    programs::TracePointContext,
    helpers::{bpf_get_current_pid_tgid, bpf_probe_read_user_buf},
};
use aya_log_ebpf::info;

// Maximum size for captured data per event
const MAX_DATA_SIZE: usize = 4096;

/// Buffer info stored between sys_enter and sys_exit
#[repr(C)]
struct RecvContext {
    buf_ptr: u64,
    buf_len: usize,
}

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

/// Target PID to filter on (set by userspace)
#[map]
static TARGET_PID: Array<u32> = Array::with_max_entries(1, 0);

/// Store buffer info from sys_enter for use in sys_exit
/// Key: tid (thread ID), Value: RecvContext
#[map]
static RECV_CONTEXTS: HashMap<u32, RecvContext> = HashMap::with_max_entries(1024, 0);

/// Tracepoint for sys_enter_recvfrom - store buffer pointer
#[tracepoint]
pub fn trace_recv_enter(ctx: TracePointContext) -> u32 {
    match try_store_recv_context(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

/// Tracepoint for sys_exit_recvfrom - capture data from buffer
#[tracepoint]
pub fn trace_recv_exit(ctx: TracePointContext) -> u32 {
    match try_capture_recv_data(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

/// Store buffer context on sys_enter
fn try_store_recv_context(ctx: TracePointContext) -> Result<u32, i64> {
    // Get current process/thread ID
    let pid_tgid = bpf_get_current_pid_tgid();
    let current_pid = (pid_tgid >> 32) as u32;
    let tid = pid_tgid as u32;

    // Check if this is our target PID
    let target_pid = TARGET_PID.get(0).ok_or(-1i64)?;
    if current_pid != *target_pid {
        return Ok(0);
    }

    // Read syscall arguments (sys_enter format)
    let fd: i32 = unsafe { ctx.read_at(16)? };
    let buf_ptr: u64 = unsafe { ctx.read_at(24)? };
    let buf_len: usize = unsafe { ctx.read_at(32)? };

    // Store context for sys_exit to use
    let context = RecvContext { buf_ptr, buf_len };
    RECV_CONTEXTS.insert(&tid, &context, 0)?;

    Ok(0)
}

/// Capture data on sys_exit using stored context
fn try_capture_recv_data(ctx: TracePointContext) -> Result<u32, i64> {
    // Get thread ID to lookup stored context
    let pid_tgid = bpf_get_current_pid_tgid();
    let tid = pid_tgid as u32;

    // Get stored buffer context from sys_enter
    let context = unsafe { RECV_CONTEXTS.get(&tid).ok_or(-1i64)? };
    let buf_ptr = context.buf_ptr;
    let buf_len = context.buf_len;

    // Clean up - remove from map
    unsafe { RECV_CONTEXTS.remove(&tid)? };

    // Check return value - if negative, recv failed
    let ret: i64 = unsafe { ctx.read_at(16)? };
    if ret <= 0 {
        return Ok(0); // recv failed or returned 0 bytes
    }

    // Use actual bytes received (min of ret and buf_len)
    let bytes_received = ret.min(buf_len as i64) as usize;

    // Limit data size to prevent exceeding stack limits
    let copy_len = if bytes_received > MAX_DATA_SIZE {
        MAX_DATA_SIZE
    } else {
        bytes_received
    };

    // Reserve space in ringbuf
    let mut event = match EVENTS.reserve::<SocketDataEvent>(0) {
        Some(e) => e,
        None => return Ok(0), // Ringbuf full, skip this event
    };

    // Get mutable reference to the event data
    let event_data = unsafe { &mut *(event.as_mut_ptr() as *mut SocketDataEvent) };

    // Populate event
    event_data.sock_id = tid as u64; // Use thread ID as socket identifier
    event_data.sport = 0;
    event_data.dport = 0;
    event_data.data_len = copy_len as u32;

    // Read data from userspace buffer
    let read_result = unsafe {
        bpf_probe_read_user_buf(
            buf_ptr as *const u8,
            &mut event_data.data[..copy_len],
        )
    };

    // Handle read error - must discard event before returning
    if let Err(_) = read_result {
        event.discard(0);
        return Ok(0);
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
