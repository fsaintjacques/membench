#[cfg(test)]
mod tests {
    use membench::record::StreamReassembler;

    #[test]
    fn test_stream_reassembler_basic() {
        let mut reassembler = StreamReassembler::new();

        // Simulate TCP packets for a single connection
        let conn_id = (("127.0.0.1", 12345), ("127.0.0.1", 11211));

        // Add packet with data "hello"
        reassembler.add_packet(conn_id, 1000, b"hello");

        // Verify we can retrieve the stream data
        let data = reassembler.get_stream(conn_id);
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_stream_reassembler_out_of_order() {
        let mut reassembler = StreamReassembler::new();
        let conn_id = (("127.0.0.1", 12345), ("127.0.0.1", 11211));

        // Add packets out of order
        reassembler.add_packet(conn_id, 2000, b"world");
        reassembler.add_packet(conn_id, 1000, b"hello");

        // Should reassemble correctly
        let data = reassembler.get_stream(conn_id);
        assert_eq!(data, b"helloworld");
    }
}
