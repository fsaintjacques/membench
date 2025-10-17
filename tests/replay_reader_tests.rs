#[cfg(test)]
mod tests {
    use membench::replay::ProfileReader;
    use membench::record::ProfileWriter;
    use membench::profile::{Event, Response, CommandType, Flags};
    use tempfile::TempDir;

    #[test]
    fn test_read_profile() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_profile.bin");
        let path = file_path.to_str().unwrap();

        // Write a profile
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

        // Read it back
        let reader = ProfileReader::new(path).unwrap();
        let metadata = reader.metadata();

        assert_eq!(metadata.total_events, 1);
        assert_eq!(metadata.unique_connections, 1);
    }
}
