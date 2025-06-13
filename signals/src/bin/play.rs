use anyhow::{Context, Result};
use num_complex::Complex32;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, StreamConfig};

use signals::ComplexOscillator;
use signals::NoiseSource;

fn main() -> Result<()> {
    println!("Hello world!");

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .context("Failed to get default output device")?;

    println!("Default output device: {}", device.name()?);

    // Obtain the device’s preferred stream configuration.
    let config = device
        .default_output_config()
        .context("failed to get default output config")?;

    println!("Default output config: {:?}", config);

    assert_eq!(
        config.sample_format(),
        SampleFormat::F32,
        "Expected sample format to be F32",
    );

    let mut osc = ComplexOscillator::new(800.0f32, 44100f32);
    let mut noise_source = NoiseSource::new(-20f32);

    let channels = config.channels() as usize;

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // react to stream events and read or write stream data here.

                // print len of data buffer
                println!("Data buffer length: {}", data.len());

                for frame in data.chunks_mut(channels) {
                    // Pull the next complex sample
                    let sample = osc.next().unwrap() + noise_source.next().unwrap();

                    frame[0] = sample.re;
                    if channels > 1 {
                        frame[1] = sample.im;
                    }
                }
            },
            move |err| {
                panic!("Stream error: {}", err);
            },
            None, // None=blocking, Some(Duration)=timeout
        )
        .context("Failed to build output stream")?;

    stream.play().context("Failed to play output stream")?;

    // SLeep for a while to let the sound play
    std::thread::sleep(std::time::Duration::from_secs(10));

    Ok(())
}
