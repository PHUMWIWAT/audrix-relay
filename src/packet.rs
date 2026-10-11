use std::error::Error;

pub const PORT: u16 = 40123;
pub const HEADER_LEN: usize = 10;
pub const HELLO: &[u8] = b"AXDR";

pub fn is_hello(data: &[u8]) -> bool {
    data.len() == 4 && data == HELLO || data.len() == 6 && data.starts_with(HELLO)
}

pub fn write_packet(buf: &mut Vec<u8>, seq: u32, sample_rate: u32, channels: u16, pcm: &[i16]) {
    buf.resize(HEADER_LEN + pcm.len() * 2, 0);
    buf[0..4].copy_from_slice(&seq.to_le_bytes());
    buf[4..8].copy_from_slice(&sample_rate.to_le_bytes());
    buf[8..10].copy_from_slice(&channels.to_le_bytes());
    for (i, sample) in pcm.iter().enumerate() {
        let start = HEADER_LEN + i * 2;
        buf[start..start + 2].copy_from_slice(&sample.to_le_bytes());
    }
}

pub fn parse_packet(data: &[u8], samples: &mut Vec<i16>) -> Result<(u32, u32, u16), Box<dyn Error>> {
    if data.len() < HEADER_LEN || (data.len() - HEADER_LEN) % 2 != 0 {
        return Err(format!("bad packet length {}", data.len()).into());
    }
    let mut seq_bytes = [0u8; 4];
    seq_bytes.copy_from_slice(&data[0..4]);
    let mut rate_bytes = [0u8; 4];
    rate_bytes.copy_from_slice(&data[4..8]);
    let mut channel_bytes = [0u8; 2];
    channel_bytes.copy_from_slice(&data[8..10]);
    let seq = u32::from_le_bytes(seq_bytes);
    let sample_rate = u32::from_le_bytes(rate_bytes);
    let channels = u16::from_le_bytes(channel_bytes);
    if sample_rate == 0 {
        return Err("packet has zero sample rate".into());
    }
    if channels == 0 {
        return Err("packet has zero channels".into());
    }
    let count = (data.len() - HEADER_LEN) / 2;
    if count % channels as usize != 0 {
        return Err("packet samples do not fill a frame".into());
    }
    samples.clear();
    samples.reserve(count);
    for i in 0..count {
        let start = HEADER_LEN + i * 2;
        let mut sample_bytes = [0u8; 2];
        sample_bytes.copy_from_slice(&data[start..start + 2]);
        samples.push(i16::from_le_bytes(sample_bytes));
    }
    Ok((seq, sample_rate, channels))
}

#[derive(Default)]
pub struct LossCount {
    next: Option<u32>,
    pub received: u32,
    pub lost: u32,
    pub duplicates: u32,
}

impl LossCount {
    pub fn observe(&mut self, seq: u32) {
        match self.next {
            None => {
                self.received += 1;
                self.next = Some(seq.wrapping_add(1));
            }
            Some(expected) if seq == expected => {
                self.received += 1;
                self.next = Some(seq.wrapping_add(1));
            }
            Some(expected) if seq > expected => {
                self.received += 1;
                self.lost += seq - expected;
                self.next = Some(seq.wrapping_add(1));
            }
            Some(_) => self.duplicates += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LossCount, is_hello, parse_packet, write_packet};

    #[test]
    fn loss_count_counts_a_gap_and_ignores_a_duplicate() {
        let mut count = LossCount::default();
        count.observe(0);
        count.observe(2);
        count.observe(1);
        assert_eq!(count.received, 2);
        assert_eq!(count.lost, 1);
        assert_eq!(count.duplicates, 1);
    }

    #[test]
    fn hello_is_only_the_four_byte_mark() {
        assert!(is_hello(b"AXDR"));
        assert!(is_hello(&[b'A', b'X', b'D', b'R', 12, 0]));
        assert!(!is_hello(b"AXDR!"));
        assert!(!is_hello(b"FIND"));
    }

    #[test]
    fn packet_roundtrip_keeps_sequence_and_samples() {
        let mut buf = Vec::new();
        write_packet(&mut buf, 7, 48_000, 2, &[1, -2, 3, 4]);
        let mut samples = Vec::new();
        let (seq, sample_rate, channels) = parse_packet(&buf, &mut samples).unwrap();
        assert_eq!(seq, 7);
        assert_eq!(sample_rate, 48_000);
        assert_eq!(channels, 2);
        assert_eq!(samples, [1, -2, 3, 4]);
    }
}
