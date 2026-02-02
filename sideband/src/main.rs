// Copyright SM6WJM 2026

use clap::Parser;

use anyhow::{Context, Result};

use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Observer, Producer, Split};

use opus::{Application, Bitrate, Encoder};

use cpal::DeviceId;
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use rtp::RtpHeader;
use std::net::{SocketAddr, UdpSocket};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about = "Real-time Opus Encoder")]
struct Args {
    /// Target UDP address (e.g., 192.168.1.50:5004)
    #[arg(short, long, default_value = "192.168.1.40:5004")]
    remote_addr: SocketAddr,

    /// Name of the audio device to use
    #[arg(short, long)]
    device: Option<String>,

    /// List available audio input devices
    #[arg(short, long, default_value_t = false)]
    list_devices: bool,

    /// Sample rate in Hz
    #[arg(short, long, default_value_t = 48000)]
    sample_rate: u32,

    /// Number of channels
    #[arg(short, long, default_value_t = 2)]
    channels: u8,

    /// Bitrate in bits per second
    #[arg(short, long, default_value_t = 96000)]
    bitrate: i32,

    /// Complexity (0-10)
    #[arg(long, default_value_t = 9)]
    complexity: i32,

    /// Enable VBR
    #[arg(long, default_value_t = true)]
    vbr: bool,

    /// Enable FEC
    #[arg(long, default_value_t = false)]
    fec: bool,
}

fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Set up the audio input stream
    let host = cpal::default_host();

    // List available input devices and exit
    if args.list_devices {
        println!("Available Input Devices:");
        let devices = host.input_devices()?;
        for (i, device) in devices.enumerate() {
            // device.id() gives the stable identifier
            // device.description() gives the human-readable name/info
            println!("{}. ID: \"{}\"", i, device.id()?);
            println!("   Name: {}\n", device.description()?);
        }
        return Ok(());
    }

    // Select the input device based on command line argument ID or default
    let device = if let Some(ref id_str) = args.device {
        // Parse the string ID back into a cpal::DeviceId
        let id: DeviceId = id_str.parse().context(
            "Failed to parse device ID. Use --list-devices to find the correct ID string.",
        )?;

        host.device_by_id(&id)
            .with_context(|| format!("Could not find device with ID: {}", id_str))?
    } else {
        host.default_input_device()
            .context("No default input device found")?
    };

    println!("Using device: \"{}\"", device.description()?);

    // Obtain the device’s preferred stream configuration.
    let default_config = device
        .default_input_config()
        .context("failed to get default input config")?;

    println!("Default input config: {:?}", default_config);

    // Only support f32 sample format for this example
    assert_eq!(
        default_config.sample_format(),
        SampleFormat::F32,
        "Expected sample format to be F32",
    );

    // Use the sample rate and channels from command line arguments
    let config: cpal::StreamConfig = cpal::StreamConfig {
        channels: args.channels as u16,
        sample_rate: args.sample_rate,
        ..default_config.into()
    };

    let channels = config.channels as usize;

    // Opus likes frames of 20ms: (sample_rate / 1000) * 20
    // double check this calculation
    let frame_size = (config.sample_rate as usize / 1000) * 20;

    // Assert that it's 960 samples per second for now
    assert!(
        frame_size == 960,
        "Expected frame size of 960 samples for 20ms at {} Hz, got {} samples",
        config.sample_rate,
        frame_size
    );

    // Supported 20 ms	160	240	320	480	960
    assert!(
        frame_size == 960
            || frame_size == 480
            || frame_size == 320
            || frame_size == 240
            || frame_size == 160,
        "Unsupported frame size: {} samples for 20ms at {} Hz",
        frame_size,
        config.sample_rate
    );

    // To opus channels
    let opus_chanels = match channels {
        1 => opus::Channels::Mono,
        2 => opus::Channels::Stereo,
        _ => anyhow::bail!("Unsupported number of channels: {}", channels),
    };

    // Create UDP socket for sending encoded data
    let socket = UdpSocket::bind("0.0.0.0:0")?;

    // Create Opus encoder
    let mut opus_encoder = Encoder::new(args.sample_rate, opus_chanels, Application::Audio)
        .context("Failed to create Opus encoder")?;
    opus_encoder.set_bitrate(Bitrate::Bits(args.bitrate))?;

    // Ring buffer for transporting audio data
    let rb = HeapRb::<f32>::new(frame_size * channels * 10);
    let (mut producer, mut consumer) = rb.split();

    // The real time audio thread
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let n = producer.push_slice(data);
                if n < data.len() {
                    eprintln!("Ring buffer overflow: dropped {} samples", data.len() - n);
                }
            },
            move |err| {
                eprintln!("Input stream error: {}", err);
            },
            None,
        )
        .context("Failed to build input stream")?;

    stream.play().context("Failed to start stream")?;
    println!("Encoding at {}bps... Press Ctrl+C to stop", args.bitrate);

    let mut input_buffer = vec![0f32; frame_size * channels];
    let mut output_packet = vec![0u8; 1472]; // Max packet size

    // RTP header
    let mut rtp_header = RtpHeader::new(96);
    // The complete RTP packet buffer
    let mut rtp_packet = vec![0u8; 1472];

    // Encoding loop
    loop {
        if consumer.occupied_len() >= input_buffer.len() {
            // Read a frame of audio data from the ring buffer
            let n = consumer.pop_slice(&mut input_buffer);
            assert_eq!(n, input_buffer.len());

            // Encode the audio data with Opus
            let encoded_bytes = opus_encoder
                .encode_float(&input_buffer, &mut output_packet)
                .context("Failed to encode audio frame")?;

            // Here you would typically send the encoded packet over the network
            println!("Encoded {} bytes", encoded_bytes);

            // Prepare RTP packet
            let header_bytes = rtp_header.serialize();
            // Copy RTP header
            rtp_packet[..header_bytes.len()].copy_from_slice(&header_bytes);
            // Copy encoded Opus data
            rtp_packet[header_bytes.len()..header_bytes.len() + encoded_bytes]
                .copy_from_slice(&output_packet[..encoded_bytes]);

            let total_size = header_bytes.len() + encoded_bytes;

            // Assert that total is below MTU
            assert!(total_size <= 1472, "RTP packet size exceeds MTU");

            socket
                .send_to(&rtp_packet[..total_size], args.remote_addr)
                .context("Failed to send RTP packet")?;

            // TODO: This is only correct for 48kHz sample rate, fix this.
            rtp_header.advance(frame_size as u32);
        } else {
            // Sleep briefly to avoid busy waiting
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
