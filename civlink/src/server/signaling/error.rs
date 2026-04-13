// Copyright SM6WJM 2026

#[derive(Debug, thiserror::Error)]
pub enum SignalingError {
    #[error("serialize signaling message: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("websocket send: {0}")]
    Socket(#[from] axum::Error),
}
