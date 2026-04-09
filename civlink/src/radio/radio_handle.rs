// Copyright SM6WJM 2026

/// Wraps the sidebridge radio connection and exposes a channel-based API
/// for use from async axum handlers.
pub struct RadioHandle {
    // TODO: sidebridge Transport + CivCodec, tokio channels for command/response
}

impl RadioHandle {
    pub fn new() -> Self {
        Self {}
    }
}
