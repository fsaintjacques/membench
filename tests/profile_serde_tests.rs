#[cfg(test)]
mod tests {
    use membench::profile::{Event, Response, CommandType, Flags};

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
            response: Response::Found(100),
        };

        let encoded = bincode::serialize(&event).expect("encode");
        let decoded: Event = bincode::deserialize(&encoded).expect("decode");

        assert_eq!(decoded.timestamp, event.timestamp);
        assert_eq!(decoded.key_hash, event.key_hash);
    }

    #[test]
    fn test_response_variants() {
        let r1 = Response::Found(500);
        let r2 = Response::NotFound;
        let r3 = Response::Error;

        let encoded1 = bincode::serialize(&r1).unwrap();
        let encoded2 = bincode::serialize(&r2).unwrap();
        let encoded3 = bincode::serialize(&r3).unwrap();

        assert_eq!(bincode::deserialize::<Response>(&encoded1).unwrap(), r1);
        assert_eq!(bincode::deserialize::<Response>(&encoded2).unwrap(), r2);
        assert_eq!(bincode::deserialize::<Response>(&encoded3).unwrap(), r3);
    }
}
