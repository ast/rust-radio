// Copyright SM6WJM 2026

use anyhow::Result;
use opus::{Channels, Decoder};

use crate::audio::AudioCapture;
use crate::Config;

const SILENCE_THRESHOLD: f32 = 0.001;

pub async fn run(config: Config) -> Result<()> {
    let capture = AudioCapture::start(config.radio.audio_device.as_deref())?;
    let sample_rate = capture.sample_rate();
    let channels = capture.channels();
    let opus_channels = match channels {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        _ => anyhow::bail!("unsupported channel count: {channels}"),
    };
    let frame_size = (sample_rate as usize / 1000) * 20;

    let mut rx = capture.subscribe();

    let mut decoder = Decoder::new(sample_rate, opus_channels)?;
    let mut decode_buf = vec![0f32; frame_size * channels as usize];

    let mut total: u64 = 0;
    let mut silent: u64 = 0;

    println!("Listening for audio frames (Ctrl+C to stop)...\n");

    let mut ctrl_c = std::pin::pin!(tokio::signal::ctrl_c());

    loop {
        tokio::select! {
            frame = rx.recv() => {
                let frame = frame?;
                total += 1;

                let samples = decoder.decode_float(&frame.data, &mut decode_buf, false)?;

                let peak = decode_buf[..samples]
                    .iter()
                    .fold(0f32, |max, &s| max.max(s.abs()));

                let is_silent = peak < SILENCE_THRESHOLD;
                if is_silent {
                    silent += 1;
                }

                let label = if is_silent { "silence" } else { "signal " };
                println!(
                    "frame {:>5} | {} bytes | {}ms | peak {:.4} | {}",
                    total,
                    frame.data.len(),
                    frame.duration_ms,
                    peak,
                    label,
                );
            }
            _ = &mut ctrl_c => {
                break;
            }
        }
    }

    println!("\n--- Summary ---");
    println!("Total frames: {total}");
    println!("Signal:       {}", total - silent);
    println!("Silent:       {silent}");

    Ok(())
}
