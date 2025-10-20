//! eBPF socket-level capture via tracepoints

use crate::record::capture::CaptureStats;
use crate::record::capture::PacketSource;
use anyhow::{Context, Result};
use aya::include_bytes_aligned;
use aya::maps::{Array, RingBuf as AyaRingBuf};
use aya::programs::TracePoint;
use aya::Ebpf;
use std::sync::mpsc;

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
    _bpf: Option<Ebpf>,                  // Holds loaded eBPF program
    rx: mpsc::Receiver<Vec<u8>>,         // Buffered channel receiver
    current_packet: Option<Vec<u8>>,     // Current packet being processed
}

impl EbpfCapture {
    pub fn new(interface: &str, _port: u16, target_pid: u32) -> Result<Self> {
        check_ebpf_capabilities()?;

        // Load eBPF bytecode
        let mut bpf = Ebpf::load(include_bytes_aligned!(concat!(
            env!("OUT_DIR"),
            "/programs"
        )))?;

        // Set target PID in BPF map
        let mut pid_map: Array<_, u32> = bpf
            .take_map("TARGET_PID")
            .context("failed to get TARGET_PID map")?
            .try_into()?;
        pid_map.set(0, target_pid, 0)?;

        tracing::info!("Set target PID filter to {}", target_pid);

        // Attach tracepoints to both enter and exit
        let enter_program: &mut TracePoint = bpf
            .program_mut("trace_recv_enter")
            .context("failed to find trace_recv_enter")?
            .try_into()?;
        enter_program.load()?;
        enter_program.attach("syscalls", "sys_enter_recvfrom")?;

        let exit_program: &mut TracePoint = bpf
            .program_mut("trace_recv_exit")
            .context("failed to find trace_recv_exit")?
            .try_into()?;
        exit_program.load()?;
        exit_program.attach("syscalls", "sys_exit_recvfrom")?;

        tracing::info!("Attached eBPF tracepoints to sys_enter/exit_recvfrom");

        // Create buffered channel for packets
        // Buffer size 1000 prevents deadlock when async task sends faster than receiver processes
        let (tx, rx) = mpsc::sync_channel(1000);

        // Spawn task to read from ringbuf
        let ringbuf: AyaRingBuf<_> = bpf
            .take_map("EVENTS")
            .context("failed to get EVENTS map")?
            .try_into()?;

        tokio::spawn(async move {
            read_events(ringbuf, tx).await;
        });

        Ok(EbpfCapture {
            interface: interface.to_string(),
            _bpf: Some(bpf),
            rx,
            current_packet: None,
        })
    }
}

/// Read events from ringbuf and send to channel
async fn read_events(
    mut ringbuf: AyaRingBuf<aya::maps::MapData>,
    tx: mpsc::SyncSender<Vec<u8>>,
) {
    loop {
        match ringbuf.next() {
            Some(event_data) => {
                tracing::debug!("Received event from ringbuf, size: {}", event_data.len());
                // Parse event
                if event_data.len() >= std::mem::size_of::<SocketDataEvent>() {
                    let event = unsafe { &*(event_data.as_ptr() as *const SocketDataEvent) };

                    let data_len = event.data_len as usize;
                    tracing::debug!("Event data_len: {}, sock_id: {}", data_len, event.sock_id);
                    if data_len > 0 && data_len <= event.data.len() {
                        let packet = event.data[..data_len].to_vec();
                        tracing::debug!("Sending packet to channel: {} bytes", packet.len());
                        let _ = tx.send(packet);
                    }
                } else {
                    tracing::warn!("Event too small: {} bytes", event_data.len());
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
        // Receive packet with timeout to allow shutdown checks
        match self.rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(packet) => {
                self.current_packet = Some(packet);
                Ok(self.current_packet.as_ref().unwrap())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout - return error to allow shutdown check
                Err(anyhow::anyhow!("Timeout waiting for packet"))
            }
            Err(_) => Err(anyhow::anyhow!("eBPF capture channel closed")),
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
