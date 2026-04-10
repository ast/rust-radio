// Copyright SM6WJM 2026

use bytes::Bytes;
use doublemap::Producer;
use opus::{Channels, Decoder};
use std::sync::mpsc;

use crate::error::CivlinkError;
use crate::Result;

/// Receives Opus packets, decodes to f32 PCM, and writes to a doublemap
/// Producer. Runs on its own thread.
pub struct AudioDecoder {
    rx: mpsc::Receiver<Bytes>,
    producer: Producer<f32>,
    decoder: Decoder,
    decode_buf: Vec<f32>,
}

impl AudioDecoder {
    pub fn new(
        rx: mpsc::Receiver<Bytes>,
        producer: Producer<f32>,
        sample_rate: u32,
        opus_channels: Channels,
        frame_samples: usize,
    ) -> Result<Self> {
        let decoder = Decoder::new(sample_rate, opus_channels)
            .map_err(|e| CivlinkError::Audio(format!("failed to create opus decoder: {e}")))?;

        Ok(Self {
            rx,
            producer,
            decoder,
            decode_buf: vec![0f32; frame_samples],
        })
    }

    /// Run the decode loop. Blocks the calling thread until the sender is dropped.
    pub fn run(&mut self) -> Result<()> {
        tracing::debug!("opus decoder started");

        loop {
            let packet = self
                .rx
                .recv()
                .map_err(|_| CivlinkError::Audio("audio playback channel disconnected".into()))?;

            let decoded_samples = self
                .decoder
                .decode_float(&packet, &mut self.decode_buf, false)
                .map_err(|e| CivlinkError::Audio(format!("opus decode error: {e}")))?;

            let samples = &self.decode_buf[..decoded_samples];

            self.producer
                .produce(|slice| {
                    let n = samples.len().min(slice.len());
                    slice[..n].copy_from_slice(&samples[..n]);
                    n
                })
                .map_err(|_| CivlinkError::Audio("audio playback ring buffer disconnected".into()))?;
        }
    }
}
