// Copyright SM6WJM 2026

use bytes::Bytes;
use doublemap::Consumer;
use opus::{Application, Channels, Encoder};
use tokio::sync::broadcast;

use super::audio_capture::AudioFrame;
use crate::error::CivlinkError;
use crate::Result;

/// Reads f32 PCM from a doublemap Consumer, Opus-encodes full frames,
/// and broadcasts them to subscribers. Runs on its own thread.
pub struct AudioEncoder {
    consumer: Consumer<f32>,
    tx: broadcast::Sender<AudioFrame>,
    encoder: Encoder,
    input_buf: Vec<f32>,
    output_buf: Vec<u8>,
}

impl AudioEncoder {
    pub fn new(
        consumer: Consumer<f32>,
        tx: broadcast::Sender<AudioFrame>,
        sample_rate: u32,
        opus_channels: Channels,
        frame_samples: usize,
    ) -> Result<Self> {
        let encoder = Encoder::new(sample_rate, opus_channels, Application::Audio)
            .map_err(|e| CivlinkError::Audio(format!("failed to create opus encoder: {e}")))?;

        Ok(Self {
            consumer,
            tx,
            encoder,
            input_buf: vec![0f32; frame_samples],
            output_buf: vec![0u8; 4000], // max opus packet
        })
    }

    /// Run the encode loop. Blocks the calling thread until the producer disconnects.
    pub fn run(&mut self) -> Result<()> {
        tracing::debug!("opus encoder started");
        let frame_samples = self.input_buf.len();

        loop {
            // Block until a full frame is available, then copy out
            self.consumer
                .consume(|slice| {
                    self.input_buf.copy_from_slice(&slice[..frame_samples]);
                    frame_samples
                })
                .map_err(|_| CivlinkError::Audio("audio capture disconnected".into()))?;

            let encoded_len = self
                .encoder
                .encode_float(&self.input_buf, &mut self.output_buf)
                .map_err(|e| CivlinkError::Audio(format!("opus encode error: {e}")))?;

            let frame = AudioFrame {
                data: Bytes::copy_from_slice(&self.output_buf[..encoded_len]),
                duration_ms: 20,
            };

            // If no subscribers, this returns Err — that's fine, just drop the frame
            let _ = self.tx.send(frame);
        }
    }
}
