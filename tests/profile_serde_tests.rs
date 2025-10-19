#[cfg(test)]
mod tests {
    use membench::profile::{CommandType, Event, Flags};

    #[test]
    fn test_event_serialization() {
        let event = Event {
            timestamp: 1000000,
            conn_id: 1,
            cmd_type: CommandType::Get,
            key_hash: 0x123456789abcdef0,
            key_size: 10,
            value_size: None,
            flags: Flags::empty(),
        };

        let encoded = bincode::serialize(&event).expect("encode");
        let decoded: Event = bincode::deserialize(&encoded).expect("decode");

        assert_eq!(decoded.timestamp, event.timestamp);
        assert_eq!(decoded.key_hash, event.key_hash);
    }
}
