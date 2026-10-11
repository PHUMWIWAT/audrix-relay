use std::error::Error;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::process::ExitCode;
use std::time::Duration;

use wasapi::{AudioCaptureClient, AudioClient, Direction, Handle, ShareMode, get_default_device, initialize_mta};

const PORT: u16 = 40123;
const PACKETS_TO_SEND: u32 = 50;
const HEADER_LEN: usize = 10;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        None => capture(),
        Some("send") => match args.next() {
            None => send(IpAddr::from([127, 0, 0, 1])),
            Some(host) => match host.parse::<IpAddr>() {
                Ok(ip) if args.next().is_none() => send(ip),
                Ok(_) => Err("send takes only an ip".into()),
                Err(_) => Err(format!("send expects an ip, got {host}").into()),
            },
        },
        Some("recv") if args.next().is_none() => recv(),
        Some("recv") => Err("recv takes no arguments".into()),
        Some(other) => Err(format!("unknown command {other}. use send or recv").into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

struct Loopback {
    client: AudioClient,
    capture: AudioCaptureClient,
    event: Handle,
    bytes: Vec<u8>,
    bytes_per_frame: usize,
    channels: u16,
    sample_rate: u32,
}

fn open_loopback() -> Result<Loopback, Box<dyn Error>> {
    initialize_mta()?;

    let device = get_default_device(&Direction::Render)?;
    let name = device.get_friendlyname()?;
    let mut client = device.get_iaudioclient()?;
    let format = client.get_mixformat()?;
    let (default_period, _) = client.get_periods()?;

    // Render device plus capture direction is WASAPI loopback inside wasapi 0.12.
    // shortcut: wasapi 0.12, the GNU toolchain has no dlltool for wasapi 0.25.
    client.initialize_client(
        &format,
        default_period,
        &Direction::Capture,
        &ShareMode::Shared,
        false,
    )?;

    let buffer_frames = client.get_bufferframecount()?;
    let sample_type = match format.get_subformat()? {
        wasapi::SampleType::Float => "float",
        wasapi::SampleType::Int => "int",
    };
    println!("device: {name}");
    println!("sample_rate: {}", format.get_samplespersec());
    println!("channels: {}", format.get_nchannels());
    println!("bits_per_sample: {}", format.get_bitspersample());
    println!("sample_type: {sample_type}");
    println!("buffer_frames: {buffer_frames}");
    println!("period_hns: {default_period}");

    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let bytes_per_frame = format.get_blockalign() as usize;
    Ok(Loopback {
        client,
        capture,
        event,
        bytes: vec![0u8; buffer_frames as usize * bytes_per_frame],
        bytes_per_frame,
        channels: format.get_nchannels(),
        sample_rate: format.get_samplespersec(),
    })
}

fn capture() -> Result<(), Box<dyn Error>> {
    let mut loopback = open_loopback()?;
    let mut frames_read = 0u32;
    let mut saw_silence = false;
    for _ in 0..4 {
        loopback.event.wait_for_event(1000)?;
        let (frames, flags) = loopback
            .capture
            .read_from_device(loopback.bytes_per_frame, &mut loopback.bytes)?;
        frames_read += frames;
        saw_silence = flags.silent;
    }
    loopback.client.stop_stream()?;

    println!("frames_read: {frames_read}");
    println!("last_packet_silent: {saw_silence}");
    if frames_read == 0 {
        return Err("loopback returned no frames".into());
    }
    Ok(())
}

fn send(ip: IpAddr) -> Result<(), Box<dyn Error>> {
    let mut loopback = open_loopback()?;
    if loopback.bytes_per_frame != loopback.channels as usize * 4 {
        return Err("mix format is not f32".into());
    }

    let dest = SocketAddr::new(ip, PORT);
    let bind_ip = if ip.is_ipv4() {
        IpAddr::from([0, 0, 0, 0])
    } else {
        IpAddr::from([0, 0, 0, 0, 0, 0, 0, 0])
    };
    let socket = UdpSocket::bind(SocketAddr::new(bind_ip, 0))?;
    let mut seq = 0u32;
    let mut pcm = Vec::new();
    let mut packet = Vec::new();
    let mut attempts = 0u32;
    while seq < PACKETS_TO_SEND {
        attempts += 1;
        if attempts > PACKETS_TO_SEND * 4 {
            return Err("capture stalled before 50 packets".into());
        }
        if !read_pcm16(&mut loopback, &mut pcm)? {
            continue;
        }
        write_packet(
            &mut packet,
            seq,
            loopback.sample_rate,
            loopback.channels,
            &pcm,
        );
        socket.send_to(&packet, dest)?;
        seq += 1;
    }
    loopback.client.stop_stream()?;
    println!("packets_sent: {seq}");
    println!("dest: {dest}");
    Ok(())
}

fn recv() -> Result<(), Box<dyn Error>> {
    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::from([0, 0, 0, 0]), PORT))?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 65536];
    let mut samples = Vec::new();
    let mut loss = LossCount::default();
    let mut peak = 0i16;
    let mut sample_rate = 0u32;
    let mut got_packet = false;

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, _)) => {
                let (seq, rate, _channels) = parse_packet(&buf[..size], &mut samples)?;
                if got_packet && rate != sample_rate {
                    return Err("sample rate changed mid stream".into());
                }
                sample_rate = rate;
                loss.observe(seq);
                for sample in &samples {
                    peak = peak.max(sample.abs());
                }
                if !got_packet {
                    socket.set_read_timeout(Some(Duration::from_millis(500)))?;
                    got_packet = true;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => break,
            Err(err) => return Err(err.into()),
        }
    }

    if !got_packet {
        return Err("no packets received".into());
    }
    println!("packets: {}", loss.received);
    println!("lost: {}", loss.lost);
    println!("duplicates: {}", loss.duplicates);
    println!("sample_rate: {sample_rate}");
    println!("peak: {peak}");
    Ok(())
}

fn read_pcm16(loopback: &mut Loopback, pcm: &mut Vec<i16>) -> Result<bool, Box<dyn Error>> {
    loopback.event.wait_for_event(1000)?;
    let (frames, flags) = loopback
        .capture
        .read_from_device(loopback.bytes_per_frame, &mut loopback.bytes)?;
    if frames == 0 {
        return Ok(false);
    }
    let channels = loopback.channels as usize;
    let count = frames as usize * channels;
    pcm.clear();
    pcm.resize(count, 0);
    if flags.silent {
        return Ok(true);
    }
    for i in 0..count {
        let start = i * 4;
        let mut raw = [0u8; 4];
        raw.copy_from_slice(&loopback.bytes[start..start + 4]);
        pcm[i] = f32_to_i16(f32::from_le_bytes(raw));
    }
    Ok(true)
}

fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

fn write_packet(buf: &mut Vec<u8>, seq: u32, sample_rate: u32, channels: u16, pcm: &[i16]) {
    buf.resize(HEADER_LEN + pcm.len() * 2, 0);
    buf[0..4].copy_from_slice(&seq.to_le_bytes());
    buf[4..8].copy_from_slice(&sample_rate.to_le_bytes());
    buf[8..10].copy_from_slice(&channels.to_le_bytes());
    for (i, sample) in pcm.iter().enumerate() {
        let start = HEADER_LEN + i * 2;
        buf[start..start + 2].copy_from_slice(&sample.to_le_bytes());
    }
}

fn parse_packet(data: &[u8], samples: &mut Vec<i16>) -> Result<(u32, u32, u16), Box<dyn Error>> {
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
struct LossCount {
    next: Option<u32>,
    received: u32,
    lost: u32,
    duplicates: u32,
}

impl LossCount {
    fn observe(&mut self, seq: u32) {
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
    use super::{LossCount, f32_to_i16, parse_packet, write_packet};

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
    fn f32_sample_clips_to_i16() {
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(-1.0), -i16::MAX);
        assert_eq!(f32_to_i16(4.0), i16::MAX);
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
