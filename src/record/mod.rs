//! Capture and anonymization logic

pub mod anonymizer;
pub mod capture;
#[cfg(feature = "ebpf")]
pub mod ebpf;
pub mod main;
pub mod parser;
pub mod writer;

pub use anonymizer::Anonymizer;
pub use capture::PacketCapture;
pub use main::run as run_record;
pub use parser::MemcacheParser;
pub use writer::ProfileWriter;
