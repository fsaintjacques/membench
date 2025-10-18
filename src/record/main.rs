//! Record command implementation

use anyhow::Result;
use std::time::SystemTime;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::record::{PacketCapture, StreamReassembler, MemcacheParser, Anonymizer, ProfileWriter};
use crate::profile::Event;

pub fn run(interface: &str, port: u16, output: &str, salt: Option<u64>) -> Result<()> {
    let salt = salt.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });

    tracing::info!("Recording from {}:{} to {}", interface, port, output);
    tracing::debug!("Salt: {}", salt);
    tracing::info!("Capturing memcache traffic... Press Ctrl+C to stop.");
    tracing::debug!("Available devices: {:?}", PacketCapture::list_devices().unwrap_or_default());

    // Initialize components
    let mut capture = PacketCapture::new(interface, port)?;
    tracing::debug!("Capture initialized on interface: {}", interface);
    let _reassembler = StreamReassembler::new();
    let parser = MemcacheParser::new();
    let anonymizer = Anonymizer::new(salt);
    let mut writer = ProfileWriter::new(output)?;

    // Set up signal handling for graceful shutdown
    let should_exit = Arc::new(AtomicBool::new(false));
    let should_exit_clone = Arc::clone(&should_exit);

    ctrlc::set_handler(move || {
        tracing::info!("Received Ctrl+C, shutting down gracefully...");
        should_exit_clone.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl+C handler");

    // Track connection state
    let mut packet_count = 0u64;
    let mut event_count = 0u64;

    tracing::info!("Capturing packets... (Press Ctrl+C to stop)");

    loop {
        // Check if we should exit
        if should_exit.load(Ordering::SeqCst) {
            tracing::info!("Shutdown signal received");
            break;
        }

        // Capture packet
        match capture.next_packet() {
            Ok(packet_data) => {
                packet_count += 1;

                // pcap returns full packets with headers. For loopback (lo0) on macOS,
                // we need to skip the link layer header (typically 14 bytes for ethernet,
                // but loopback has a different format)
                // Try to find memcache protocol markers to skip headers
                let payload = if let Some(pos) = packet_data.windows(2).position(|w| w == b"\r\n") {
                    // Found \r\n which suggests we're at or near application data
                    // Search backwards for command start (GET, SET, etc.)
                    if let Some(cmd_start) = packet_data[..pos].windows(3).rposition(|w| {
                        w == b"get" || w == b"set" || w == b"del" || w == b"noo"
                    }) {
                        &packet_data[cmd_start..]
                    } else {
                        packet_data
                    }
                } else {
                    packet_data
                };

                // Try to parse as memcache command
                if let Ok(data_str) = std::str::from_utf8(payload) {
                    if data_str.contains('\r') && data_str.contains('\n') {
                        // Try parsing as a command
                        match parser.parse_command(payload) {
                            Ok((cmd, _)) => {
                                // Extract the actual key from the payload
                                let key_bytes = &payload[cmd.key_range.clone()];
                                let key_size = cmd.key_range.len() as u32;

                                // Create event from parsed command
                                let event = Event {
                                    timestamp: SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap()
                                        .as_micros() as u64,
                                    conn_id: (packet_count % 32) as u32, // Simplified connection ID
                                    cmd_type: cmd.cmd_type,
                                    key_hash: anonymizer.hash_key(key_bytes), // Hash the actual key
                                    key_size,
                                    value_size: cmd.value_size,
                                    flags: cmd.flags,
                                };

                                writer.write_event(&event)?;
                                event_count += 1;

                                if packet_count % 1000 == 0 {
                                    tracing::info!("Captured {} packets, {} events", packet_count, event_count);
                                }
                            }
                            Err(e) => {
                                if packet_count <= 10 {
                                    let data_preview = String::from_utf8_lossy(packet_data);
                                    let preview = if data_preview.len() > 100 {
                                        format!("{}...", &data_preview[..100])
                                    } else {
                                        data_preview.to_string()
                                    };
                                    tracing::debug!("Parse error on packet {}: {} | Data (len={}): {:?}", packet_count, e, packet_data.len(), preview);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // Timeout or other error - just continue
                continue;
            }
        }
    }

    // Finalize profile
    tracing::info!("Finalizing profile...");
    writer.finish()?;

    tracing::info!("✓ Recording complete");
    tracing::info!("  Profile: {}", output);
    tracing::info!("  Packets captured: {}", packet_count);
    tracing::info!("  Events recorded: {}", event_count);

    Ok(())
}
