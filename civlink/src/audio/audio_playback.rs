// Copyright SM6WJM 2026

use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use doublemap::ring_buffer_pair;
use opus::Channels;
use std::sync::mpsc;

use super::audio_decoder::AudioDecoder;
use crate::error::CivlinkError;
use crate::Result;

/// Receives Opus-encoded audio, decodes, and plays back through a cpal
/// output device (for TX audio to the radio).
pub struct AudioPlayback {
    tx: mpsc::SyncSender<Bytes>,
    _stream: cpal::Stream,
}

impl AudioPlayback {
    /// Start playback on the specified device (or default if None).
    /// Returns the AudioPlayback handle; send Opus packets via [`send`].
    pub fn start(device_name: Option<&str>, sample_rate: u32, channels: u16) -> Result<Self> {
        let host = cpal::default_host();

        let device = match device_name {
            Some(name) => find_device(&host, name)?,
            None => host
                .default_output_device()
                .ok_or_else(|| CivlinkError::Audio("no default output device".into()))?,
        };

        let device_desc = device
            .description()
            .map(|d| d.to_string())
            .unwrap_or_else(|_| "unknown".into());
        tracing::info!("audio playback device: {device_desc}");

        let default_config = device.default_output_config()?;

        tracing::debug!("default output config: {default_config:?}");

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
            "audio playback: {}Hz, {} ch, frame_size={} samples",
            sample_rate,
            channels,
            frame_size
        );

        // Ring buffer: consumer_required=1 so cpal callback only blocks when
        // completely empty (effectively non-blocking with 8x headroom).
        // producer_required=frame_samples so decoder wakes per opus frame.
        let (producer, consumer) = ring_buffer_pair::<f32>(frame_samples, 1);

        // Bounded channel for incoming opus packets (~500ms at 20ms frames)
        let (tx, rx) = mpsc::sync_channel::<Bytes>(25);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let result = consumer.consume(|slice| {
                    let n = data.len().min(slice.len());
                    data[..n].copy_from_slice(&slice[..n]);
                    // Zero-fill any remaining output to avoid stale audio
                    for sample in &mut data[n..] {
                        *sample = 0.0;
                    }
                    n
                });
                if result.is_err() {
                    // Decoder disconnected — output silence
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }
            },
            move |err| {
                tracing::error!("audio output stream error: {err}");
            },
            None,
        )?;

        stream.play()?;

        tracing::info!("audio playback stream started");

        // Spawn the decoding thread (not async — opus decoding is CPU-bound)
        let mut decoder = AudioDecoder::new(rx, producer, sample_rate, opus_channels, frame_samples)?;
        std::thread::Builder::new()
            .name("audio-decoder".into())
            .spawn(move || {
                if let Err(e) = decoder.run() {
                    tracing::error!("audio decoder exited with error: {e}");
                }
            })?;

        Ok(Self { tx, _stream: stream })
    }

    /// Send an Opus-encoded packet for decoding and playback.
    pub fn send(&self, packet: Bytes) -> Result<()> {
        self.tx
            .try_send(packet)
            .map_err(|_| CivlinkError::Audio("audio playback channel full or disconnected".into()))
    }
}

fn find_device(host: &cpal::Host, name: &str) -> Result<cpal::Device> {
    let devices = host.output_devices()?;

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
