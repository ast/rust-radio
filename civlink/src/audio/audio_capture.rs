// Copyright SM6WJM 2026

/// Captures audio from a cpal device, encodes with Opus, and sends
/// encoded frames via a channel for WebRTC media track consumption.
pub struct AudioCapture {
    // TODO: cpal stream, ringbuf, opus encoder, tokio sender
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {}
    }
}
