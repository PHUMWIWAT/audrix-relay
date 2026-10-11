use std::error::Error;
use std::process::ExitCode;

use wasapi::{Direction, ShareMode, get_default_device, initialize_mta};

fn main() -> ExitCode {
    match capture() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("capture failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn capture() -> Result<(), Box<dyn Error>> {
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
    let capture_client = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let bytes_per_frame = format.get_blockalign() as usize;
    let mut bytes = vec![0u8; buffer_frames as usize * bytes_per_frame];
    let mut frames_read = 0u32;
    let mut saw_silence = false;
    for _ in 0..4 {
        event.wait_for_event(1000)?;
        let (frames, flags) = capture_client.read_from_device(bytes_per_frame, &mut bytes)?;
        frames_read += frames;
        saw_silence = flags.silent;
    }
    client.stop_stream()?;

    println!("frames_read: {frames_read}");
    println!("last_packet_silent: {saw_silence}");
    if frames_read == 0 {
        return Err("loopback returned no frames".into());
    }
    Ok(())
}
