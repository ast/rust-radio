// Copyright SM6WJM 2026

use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use doublemap::ring_buffer_pair;
use opus::Channels;
use tokio::sync::broadcast;

use super::audio_encoder::AudioEncoder;

/// An Opus-encoded audio frame with its duration.
#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub data: Bytes,
    pub duration_ms: u32,
}
use crate::error::CivlinkError;
use crate::Result;

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
        let frame_samples = frame_size * ch;

        tracing::info!(
            "audio capture: {}Hz, {} ch, frame_size={} samples",
            sample_rate,
            channels,
            frame_size
        );

        // Ring buffer: producer_required=1 so cpal callback only blocks when
        // completely full (effectively non-blocking with 8x headroom).
        // consumer_required=frame_samples so encoder wakes per opus frame.
        let (producer, consumer) = ring_buffer_pair::<f32>(1, frame_samples);

        // Broadcast channel for encoded frames (capacity for ~500ms of audio at 20ms frames)
        let (tx, _) = broadcast::channel::<AudioFrame>(25);
        let tx_clone = tx.clone();

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let result = producer.produce(|slice| {
                        let n = data.len().min(slice.len());
                        slice[..n].copy_from_slice(&data[..n]);
                        n
                    });
                    if result.is_err() {
                        tracing::warn!("audio ring buffer disconnected");
                    }
                },
                move |err| {
                    tracing::error!("audio input stream error: {err}");
                },
                None,
            )
            .map_err(|e| CivlinkError::Audio(format!("failed to build input stream: {e}")))?;

        stream
            .play()
            .map_err(|e| CivlinkError::Audio(format!("failed to start stream: {e}")))?;

        tracing::info!("audio capture stream started");

        // Spawn the encoding thread (not async — opus encoding is CPU-bound)
        let mut encoder = AudioEncoder::new(consumer, tx_clone, sample_rate, opus_channels, frame_samples)?;
        std::thread::Builder::new()
            .name("audio-encoder".into())
            .spawn(move || {
                if let Err(e) = encoder.run() {
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
