#[cfg(test)]
mod tests {
    use membench::record::ProfileWriter;
    use membench::profile::{Event, Response, CommandType, Flags};
    use tempfile::NamedTempFile;

    #[test]
    fn test_write_profile() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();

        let mut writer = ProfileWriter::new(path).unwrap();

        let event = Event {
            timestamp: 1000,
            conn_id: 1,
            cmd_type: CommandType::Get,
            key_hash: 0x123456789,
            key_size: 10,
            value_size: None,
            flags: Flags::empty(),
            response: Response::Found(100),
        };

        writer.write_event(&event).unwrap();
        writer.finish().unwrap();

        // Verify file was written and has content
        let metadata = std::fs::metadata(path).unwrap();
        assert!(metadata.len() > 0);
    }
}
