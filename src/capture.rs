use std::error::Error;

use wasapi::{
    AudioCaptureClient, AudioClient, Direction, Handle, ShareMode, get_default_device, initialize_mta,
};

pub struct Loopback {
    pub client: AudioClient,
    pub capture: AudioCaptureClient,
    pub event: Handle,
    pub bytes: Vec<u8>,
    pub bytes_per_frame: usize,
    pub channels: u16,
    pub sample_rate: u32,
    pub device_name: String,
}

pub fn open_loopback() -> Result<Loopback, Box<dyn Error>> {
    initialize_mta()?;

    let device = get_default_device(&Direction::Render)?;
    let name = device.get_friendlyname()?;
    let mut client = device.get_iaudioclient()?;
    let format = client.get_mixformat()?;
    let (default_period, min_period) = client.get_periods()?;
    let period = if min_period > 0 {
        min_period.clamp(100_000, default_period.max(100_000))
    } else {
        default_period
    };

    // Render device plus capture direction is WASAPI loopback inside wasapi 0.12.
    // shortcut: wasapi 0.12, the GNU toolchain has no dlltool for wasapi 0.25.
    client.initialize_client(
        &format,
        period,
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
    println!("period_hns: {period}");

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
        device_name: name,
    })
}

pub fn read_pcm16(loopback: &mut Loopback, pcm: &mut Vec<i16>) -> Result<bool, Box<dyn Error>> {
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

pub fn f32_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32) as i16
}

pub fn capture() -> Result<(), Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::f32_to_i16;

    #[test]
    fn f32_sample_clips_to_i16() {
        assert_eq!(f32_to_i16(0.0), 0);
        assert_eq!(f32_to_i16(1.0), i16::MAX);
        assert_eq!(f32_to_i16(-1.0), -i16::MAX);
        assert_eq!(f32_to_i16(4.0), i16::MAX);
    }
}
