// Copyright SM6WJM 2026

/// Receives Opus-encoded audio from WebRTC, decodes, and plays back
/// through a cpal output device (for TX audio to the radio).
pub struct AudioPlayback {
    // TODO: opus decoder, ringbuf, cpal output stream
}

impl AudioPlayback {
    pub fn new() -> Self {
        Self {}
    }
}
