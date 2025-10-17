use std::collections::HashMap;

type ConnKey = ((&'static str, u16), (&'static str, u16));

pub struct StreamReassembler {
    streams: HashMap<ConnKey, StreamBuffer>,
}

struct StreamBuffer {
    segments: Vec<(u32, Vec<u8>)>,
}

impl StreamReassembler {
    pub fn new() -> Self {
        StreamReassembler {
            streams: HashMap::new(),
        }
    }

    pub fn add_packet(&mut self, conn_id: ConnKey, seq: u32, data: &[u8]) {
        let buffer = self.streams.entry(conn_id).or_insert_with(|| StreamBuffer {
            segments: Vec::new(),
        });

        buffer.segments.push((seq, data.to_vec()));
        buffer.segments.sort_by_key(|(seq, _)| *seq);
    }

    pub fn get_stream(&self, conn_id: ConnKey) -> Vec<u8> {
        if let Some(buffer) = self.streams.get(&conn_id) {
            buffer.segments.iter().flat_map(|(_, data)| data.iter().cloned()).collect()
        } else {
            Vec::new()
        }
    }
}
