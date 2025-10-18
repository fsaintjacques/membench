#![no_std]
#![no_main]

use aya_ebpf::{macros::*, maps::RingBuf, programs::SockOpsContext};
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

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
