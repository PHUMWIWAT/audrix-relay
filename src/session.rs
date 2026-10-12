use std::collections::VecDeque;
use std::error::Error;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::capture;
use crate::packet::{self, LossCount, PORT};
use crate::ui;

const PACKETS_TO_SEND: u32 = 50;
// A missed keepalive must not end the session. The phone retries; only a
// long silence means it is actually gone.
const TARGET_TIMEOUT: Duration = Duration::from_secs(30);

pub fn serve() -> Result<(), Box<dyn Error>> {
    let ip = primary_ipv4()?;
    ui::set_ip(ip.to_string());
    let running = Arc::new(AtomicBool::new(true));
    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::from([0, 0, 0, 0]), PORT)).map_err(|err| {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            "โปรแกรมเปิดอยู่แล้ว ปิดหน้าต่างเดิมก่อน".into()
        } else {
            Box::new(err) as Box<dyn Error>
        }
    })?;
    socket.set_read_timeout(Some(Duration::from_millis(200)))?;
    let socket = Arc::new(socket);
    let audio = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
    let target = Arc::new(Mutex::new(None::<(SocketAddr, Instant)>));

    let listen_socket = Arc::clone(&socket);
    let listen_target = Arc::clone(&target);
    let listen_running = Arc::clone(&running);
    std::thread::spawn(move || listen_for_phones(listen_socket, listen_target, listen_running));

    let send_socket = audio;
    let send_target = Arc::clone(&target);
    let send_running = Arc::clone(&running);
    std::thread::spawn(move || {
        if let Err(err) = capture_send(send_socket, send_target, send_running) {
            ui::set_error(err.to_string());
        }
    });

    ui::run();
    running.store(false, Ordering::Relaxed);
    Ok(())
}

fn capture_send(
    socket: Arc<UdpSocket>,
    target: Arc<Mutex<Option<(SocketAddr, Instant)>>>,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error>> {
    let mut loopback = capture::open_loopback()?;
    ui::set_device(loopback.device_name.clone());
    if loopback.bytes_per_frame != loopback.channels as usize * 4 {
        return Err("mix format is not f32".into());
    }
    let mut seq = 0u32;
    let mut pcm = Vec::new();
    let mut packet = Vec::new();
    let mut queued: VecDeque<u8> = VecDeque::new();
    let mut active: Option<SocketAddr> = None;
    let bytes_per_frame = (loopback.channels as usize).max(1) * 2;
    while running.load(Ordering::Relaxed) {
        match capture::read_pcm16(&mut loopback, &mut pcm) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(err) if err.to_string().contains("Wait timed out") => continue,
            Err(err) => return Err(err),
        }
        for sample in &pcm {
            queued.extend(sample.to_le_bytes());
        }
        let chunk_bytes = chunk_bytes(loopback.sample_rate, bytes_per_frame);
        let cap = chunk_bytes.saturating_mul(4);
        while cap > 0 && queued.len() > cap {
            queued.drain(..bytes_per_frame.min(queued.len()));
        }
        let dest = current_target(&target);
        let Some(dest) = dest else {
            if active.is_some() {
                ui::set_peer(None);
                ui::set_rtt(None);
            }
            active = None;
            queued.clear();
            continue;
        };
        if active != Some(dest) {
            seq = 0;
            active = Some(dest);
            ui::set_peer(Some(dest.ip().to_string()));
        }
        while chunk_bytes > 0 && queued.len() >= chunk_bytes {
            let chunk: Vec<u8> = queued.drain(..chunk_bytes).collect();
            seq = send_chunk(
                &socket,
                dest,
                seq,
                loopback.sample_rate,
                loopback.channels,
                &chunk,
                &mut packet,
            )?;
        }
    }
    loopback.client.stop_stream()?;
    Ok(())
}

fn listen_for_phones(
    socket: Arc<UdpSocket>,
    target: Arc<Mutex<Option<(SocketAddr, Instant)>>>,
    running: Arc<AtomicBool>,
) {
    let mut buf = [0u8; 256];
    while running.load(Ordering::Relaxed) {
        let Ok((size, addr)) = socket.recv_from(&mut buf) else {
            continue;
        };
        let data = &buf[..size];
        if data == b"FIND" {
            let mut msg = b"HERE".to_vec();
            let name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "PC".into());
            msg.extend(name.into_bytes());
            let _ = socket.send_to(&msg, addr);
            continue;
        }
        if data.len() == 8 && data.starts_with(b"PING") {
            let mut pong = [0u8; 8];
            pong[..4].copy_from_slice(b"PONG");
            pong[4..].copy_from_slice(&data[4..]);
            let _ = socket.send_to(&pong, addr);
            continue;
        }
        if packet::is_hello(data) {
            let mut guard = lock_target(&target);
            *guard = Some((addr, Instant::now()));
            if data.len() == 6 {
                let ms = u16::from_le_bytes([data[4], data[5]]) as u32;
                ui::set_rtt(Some(ms));
            }
            ui::set_peer(Some(addr.ip().to_string()));
        }
    }
}

fn chunk_bytes(sample_rate: u32, bytes_per_frame: usize) -> usize {
    let ms = capture::chunk_ms();
    (sample_rate as usize)
        .saturating_mul(bytes_per_frame)
        .saturating_mul(ms as usize)
        / 1000
}

fn send_chunk(
    socket: &UdpSocket,
    dest: SocketAddr,
    mut seq: u32,
    sample_rate: u32,
    channels: u16,
    chunk: &[u8],
    packet: &mut Vec<u8>,
) -> Result<u32, Box<dyn Error>> {
    let bytes_per_frame = (channels as usize).max(1) * 2;
    let max_payload = packet::max_payload_bytes(bytes_per_frame);
    let per = packet::even_split_bytes(chunk.len(), max_payload, bytes_per_frame).max(1);
    for slice in chunk.chunks(per) {
        packet::write_pcm_bytes(packet, seq, sample_rate, channels, slice);
        socket.send_to(packet, dest)?;
        seq = seq.wrapping_add(1);
    }
    Ok(seq)
}

fn current_target(target: &Mutex<Option<(SocketAddr, Instant)>>) -> Option<SocketAddr> {
    let mut guard = lock_target(target);
    match *guard {
        Some((addr, seen)) if seen.elapsed() < TARGET_TIMEOUT => Some(addr),
        Some(_) => {
            *guard = None;
            println!("มือถือหลุด");
            None
        }
        None => None,
    }
}

fn lock_target(
    target: &Mutex<Option<(SocketAddr, Instant)>>,
) -> std::sync::MutexGuard<'_, Option<(SocketAddr, Instant)>> {
    target.lock().unwrap_or_else(|err| err.into_inner())
}

fn primary_ipv4() -> Result<IpAddr, Box<dyn Error>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip())
}

pub fn send(ip: IpAddr, limit: Option<u32>) -> Result<(), Box<dyn Error>> {
    let mut loopback = capture::open_loopback()?;
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
    let mut packet_buf = Vec::new();
    let mut attempts = 0u32;
    let mut said_waiting = false;
    println!("ส่งไป {dest}");
    if limit.is_none() {
        println!("กด Ctrl+C เพื่อหยุด");
    }
    while limit.is_none_or(|max| seq < max) {
        if limit.is_some() {
            attempts += 1;
            if attempts > PACKETS_TO_SEND * 4 {
                return Err("capture stalled before 50 packets".into());
            }
        }
        match capture::read_pcm16(&mut loopback, &mut pcm) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(err) if limit.is_none() && err.to_string().contains("Wait timed out") => {
                if !said_waiting {
                    println!("ยังไม่มีเสียงจากอุปกรณ์ เปิดเสียงบน PC แล้วรอ");
                    said_waiting = true;
                }
                continue;
            }
            Err(err) => return Err(err),
        }
        packet::write_packet(
            &mut packet_buf,
            seq,
            loopback.sample_rate,
            loopback.channels,
            &pcm,
        );
        socket.send_to(&packet_buf, dest)?;
        seq += 1;
        if seq.is_multiple_of(50) {
            println!("ส่งแล้ว {seq} แพ็กเก็ต");
        }
    }
    loopback.client.stop_stream()?;
    println!("packets_sent: {seq}");
    println!("dest: {dest}");
    Ok(())
}

pub fn recv() -> Result<(), Box<dyn Error>> {
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
                let (seq, rate, _channels) = packet::parse_packet(&buf[..size], &mut samples)?;
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
