#[cfg(test)]
mod tests {
    use membench::record::MemcacheParser;
    use membench::profile::{CommandType, Response};

    #[test]
    fn test_parse_get_request() {
        let input = b"mg testkey v\r\n";
        let parser = MemcacheParser::new();

        let (cmd, _rest) = parser.parse_command(input).unwrap();
        assert_eq!(cmd.cmd_type, CommandType::Get);
        assert_eq!(cmd.key_range, 3..10); // "testkey"
    }

    #[test]
    fn test_parse_set_request() {
        let input = b"ms mykey 5\r\nhello\r\n";
        let parser = MemcacheParser::new();

        let (cmd, _rest) = parser.parse_command(input).unwrap();
        assert_eq!(cmd.cmd_type, CommandType::Set);
        assert_eq!(cmd.value_size, Some(5));
    }

    #[test]
    fn test_parse_response_found() {
        let input = b"VA 5\r\nhello\r\n";
        let parser = MemcacheParser::new();

        let response = parser.parse_response(input).unwrap();
        match response.resp {
            Response::Found(size) => assert_eq!(size, 5),
            _ => panic!("expected Found"),
        }
    }

    #[test]
    fn test_parse_response_not_found() {
        let input = b"EN\r\n";
        let parser = MemcacheParser::new();

        let response = parser.parse_response(input).unwrap();
        assert_eq!(response.resp, Response::NotFound);
    }
}
