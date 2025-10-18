//! eBPF packet capture backend

#[cfg(feature = "ebpf")]
pub mod programs;

#[cfg(feature = "ebpf")]
pub use programs::EbpfCapture;
