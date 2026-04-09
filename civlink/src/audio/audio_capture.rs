// Copyright SM6WJM 2026

use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use opus::{Application, Channels, Encoder};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};
use tokio::sync::broadcast;

use crate::error::CivlinkError;
use crate::Result;

/// An Opus-encoded audio frame with its duration.
#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub data: Bytes,
    pub duration_ms: u32,
}

/// Captures audio from a cpal device, encodes with Opus, and broadcasts
/// encoded frames to all subscribers via a tokio broadcast channel.
pub struct AudioCapture {
    tx: broadcast::Sender<AudioFrame>,
    _stream: cpal::Stream,
}

impl AudioCapture {
    /// Start capturing audio from the specified device (or default if None).
    /// Returns the AudioCapture handle and keeps streaming until dropped.
    pub fn start(device_name: Option<&str>, sample_rate: u32, channels: u16) -> Result<Self> {
        let host = cpal::default_host();

        let device = match device_name {
            Some(name) => find_device(&host, name)?,
            None => host
                .default_input_device()
                .ok_or_else(|| CivlinkError::Audio("no default input device".into()))?,
        };

        let device_desc = device
            .description()
            .map(|d| d.to_string())
            .unwrap_or_else(|_| "unknown".into());
        tracing::info!("audio capture device: {device_desc}");

        let default_config = device
            .default_input_config()
            .map_err(|e| CivlinkError::Audio(format!("failed to get input config: {e}")))?;

        let sample_format = default_config.sample_format();
        tracing::debug!("default input config: {default_config:?}");

        let config = StreamConfig {
            channels,
            sample_rate,
            ..default_config.into()
        };

        let opus_channels = match channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(CivlinkError::Audio(format!("unsupported channel count: {channels}"))),
        };

        // 20ms frame size: (sample_rate / 1000) * 20
        let frame_size = (sample_rate as usize / 1000) * 20;
        let ch = channels as usize;

        tracing::info!(
            "audio capture: {}Hz, {} ch, format={:?}, frame_size={} samples",
            sample_rate,
            channels,
            sample_format,
            frame_size
        );

        // Ring buffer: 10 frames of headroom
        let rb = HeapRb::<f32>::new(frame_size * ch * 10);
        let (mut producer, mut consumer) = rb.split();

        // Broadcast channel for encoded frames (capacity for ~500ms of audio at 20ms frames)
        let (tx, _) = broadcast::channel::<AudioFrame>(25);
        let tx_clone = tx.clone();

        // Build the cpal input stream, converting to f32 regardless of device format
        let stream = match sample_format {
            SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let n = producer.push_slice(data);
                        if n < data.len() {
                            tracing::warn!("audio ring buffer overflow: dropped {} samples", data.len() - n);
                        }
                    },
                    move |err| {
                        tracing::error!("audio input stream error: {err}");
                    },
                    None,
                )
                .map_err(|e| CivlinkError::Audio(format!("failed to build input stream: {e}")))?,
            SampleFormat::I16 => device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        for &sample in data {
                            let f = sample as f32 / i16::MAX as f32;
                            let _ = producer.try_push(f);
                        }
                    },
                    move |err| {
                        tracing::error!("audio input stream error: {err}");
                    },
                    None,
                )
                .map_err(|e| CivlinkError::Audio(format!("failed to build input stream: {e}")))?,
            _ => {
                return Err(CivlinkError::Audio(format!(
                    "unsupported sample format: {sample_format:?}"
                )));
            }
        };

        stream
            .play()
            .map_err(|e| CivlinkError::Audio(format!("failed to start stream: {e}")))?;

        tracing::info!("audio capture stream started");

        // Spawn the encoding thread (not async — opus encoding is CPU-bound)
        std::thread::Builder::new()
            .name("audio-encoder".into())
            .spawn(move || {
                if let Err(e) = encode_loop(&mut consumer, &tx_clone, sample_rate, opus_channels, frame_size, ch) {
                    tracing::error!("audio encoder exited with error: {e}");
                }
            })
            .map_err(|e| CivlinkError::Audio(format!("failed to spawn encoder thread: {e}")))?;

        Ok(Self { tx, _stream: stream })
    }

    /// Subscribe to the audio stream. Returns a receiver that yields Opus-encoded frames.
    pub fn subscribe(&self) -> broadcast::Receiver<AudioFrame> {
        self.tx.subscribe()
    }
}

fn find_device(host: &cpal::Host, name: &str) -> Result<cpal::Device> {
    let devices = host
        .input_devices()
        .map_err(|e| CivlinkError::Audio(format!("failed to enumerate devices: {e}")))?;

    for device in devices {
        let desc = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_default();
        if desc.contains(name) {
            return Ok(device);
        }
    }

    Err(CivlinkError::Audio(format!("audio device '{name}' not found")))
}

fn encode_loop(
    consumer: &mut impl Consumer<Item = f32>,
    tx: &broadcast::Sender<AudioFrame>,
    sample_rate: u32,
    opus_channels: Channels,
    frame_size: usize,
    channels: usize,
) -> Result<()> {
    let mut encoder = Encoder::new(sample_rate, opus_channels, Application::Audio)
        .map_err(|e| CivlinkError::Audio(format!("failed to create opus encoder: {e}")))?;

    let mut input_buf = vec![0f32; frame_size * channels];
    let mut output_buf = vec![0u8; 4000]; // max opus packet

    tracing::debug!("opus encoder started");

    loop {
        if consumer.occupied_len() >= input_buf.len() {
            let n = consumer.pop_slice(&mut input_buf);
            debug_assert_eq!(n, input_buf.len());

            let encoded_len = encoder
                .encode_float(&input_buf, &mut output_buf)
                .map_err(|e| CivlinkError::Audio(format!("opus encode error: {e}")))?;

            let frame = AudioFrame {
                data: Bytes::copy_from_slice(&output_buf[..encoded_len]),
                duration_ms: 20,
            };

            // If no subscribers, this returns Err — that's fine, just drop the frame
            let _ = tx.send(frame);
        } else {
            // Sleep briefly to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
