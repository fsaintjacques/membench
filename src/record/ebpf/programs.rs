//! eBPF socket-level capture via tracepoints

<<<<<<< HEAD
use crate::record::capture::CaptureStats;
use crate::record::capture::PacketSource;
use anyhow::Result;
use aya::Ebpf;
=======
use anyhow::{Result, Context as _};
use aya::{Ebpf, maps::RingBuf as AyaRingBuf, programs::TracePoint};
use aya::util::online_cpus;
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::record::capture::{CaptureStats, PacketSource};
>>>>>>> 4c3e21c (refactor: update EbpfCapture for socket ringbuf capture)

/// Event structure matching kernel-side definition
#[repr(C)]
struct SocketDataEvent {
    sock_id: u64,
    sport: u16,
    dport: u16,
    data_len: u32,
    data: [u8; 4096],
}

/// eBPF socket capture using tracepoints
pub struct EbpfCapture {
    interface: String,
    port: u16,
<<<<<<< HEAD
    _bpf: Option<Ebpf>,           // Holds loaded eBPF program
    packets_buffer: Vec<Vec<u8>>, // Buffered packets
}

impl EbpfCapture {
    /// Load and attach eBPF program for packet capture
    ///
    /// This creates a TC ingress hook on the specified interface
    /// to filter and capture packets destined for port 11211.
    ///
    /// # Errors
    /// Returns error if eBPF program cannot be loaded or attached.
    /// Requires CAP_BPF and CAP_PERFMON capabilities (or CAP_SYS_ADMIN).
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        check_ebpf_capabilities()?;

        // TODO: Load eBPF program from embedded bytecode
        // TODO: Attach to interface TC ingress
        // TODO: Open perf buffer for reading

        Ok(EbpfCapture {
            interface: interface.to_string(),
            port,
            _bpf: None, // TODO: Load program
            packets_buffer: Vec::new(),
        })
    }
}

impl PacketSource for EbpfCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        // TODO: Read from perf buffer
        // For now, return error to prevent infinite loop
        Err(anyhow::anyhow!("eBPF packet reading not yet implemented"))
    }

    fn source_info(&self) -> &str {
        &self.interface
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn stats(&mut self) -> Option<CaptureStats> {
        None
    }
=======
    _bpf: Ebpf,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    current_packet: Option<Vec<u8>>,
>>>>>>> 4c3e21c (refactor: update EbpfCapture for socket ringbuf capture)
}
