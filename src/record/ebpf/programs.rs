//! eBPF socket-level capture via tracepoints

<<<<<<< HEAD
use crate::record::capture::CaptureStats;
use crate::record::capture::PacketSource;
use anyhow::Result;
use aya::Ebpf;
=======
use anyhow::{Result, Context as _};
use aya::{Ebpf, include_bytes_aligned, maps::RingBuf as AyaRingBuf, programs::TracePoint};
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

impl EbpfCapture {
    pub fn new(interface: &str, port: u16) -> Result<Self> {
        check_ebpf_capabilities()?;

        // Load eBPF bytecode
        let mut bpf = Ebpf::load(include_bytes_aligned!(
            concat!(env!("OUT_DIR"), "/programs")
        ))?;

        // Attach tracepoint to sys_enter_recvfrom
        let program: &mut TracePoint = bpf
            .program_mut("trace_recv_enter")
            .context("failed to find trace_recv_enter")?
            .try_into()?;
        program.load()?;
        program.attach("syscalls", "sys_enter_recvfrom")?;

        tracing::info!("Attached eBPF tracepoint to sys_enter_recvfrom");

        // Create channel for packets
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn task to read from ringbuf
        let ringbuf: AyaRingBuf<_> = bpf.take_map("EVENTS")
            .context("failed to get EVENTS map")?
            .try_into()?;

        tokio::spawn(async move {
            read_events(ringbuf, tx).await;
        });

        Ok(EbpfCapture {
            interface: interface.to_string(),
            port,
            _bpf: bpf,
            rx,
            current_packet: None,
        })
    }
}

/// Read events from ringbuf and send to channel
async fn read_events(mut ringbuf: AyaRingBuf<aya::maps::MapData>, tx: mpsc::UnboundedSender<Vec<u8>>) {
    loop {
        match ringbuf.next() {
            Some(event_data) => {
                // Parse event
                if event_data.len() >= std::mem::size_of::<SocketDataEvent>() {
                    let event = unsafe {
                        &*(event_data.as_ptr() as *const SocketDataEvent)
                    };

                    let data_len = event.data_len as usize;
                    if data_len > 0 && data_len <= event.data.len() {
                        let packet = event.data[..data_len].to_vec();
                        let _ = tx.send(packet);
                    }
                }
            }
            None => {
                // No data available, yield to allow other tasks to run
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        }
    }
}

impl PacketSource for EbpfCapture {
    fn next_packet(&mut self) -> Result<&[u8]> {
        // Try to receive packet from channel (blocking)
        match self.rx.blocking_recv() {
            Some(packet) => {
                self.current_packet = Some(packet);
                Ok(self.current_packet.as_ref().unwrap())
            }
            None => Err(anyhow::anyhow!("eBPF capture channel closed")),
        }
    }

    fn source_info(&self) -> &str {
        &self.interface
    }

    fn is_finite(&self) -> bool {
        false // Socket capture is continuous
    }

    fn stats(&mut self) -> Option<CaptureStats> {
        None // TODO: Implement stats tracking
    }
}

#[cfg(target_os = "linux")]
fn check_ebpf_capabilities() -> Result<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn check_ebpf_capabilities() -> Result<()> {
    Err(anyhow::anyhow!("eBPF only supported on Linux"))
}
