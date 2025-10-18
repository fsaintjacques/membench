#![no_std]
#![no_main]

// Stub file - socket-level eBPF implementation coming in next commits
//
// This file previously contained TC (Traffic Control) ingress hooks for
// packet-level capture. We're pivoting to socket-level capture using
// sockops/sockmap for more efficient and protocol-aware capture.

use core::panic::PanicInfo;

/// Panic handler required for no_std eBPF programs
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
