// Copyright SM6WJM 2026

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CivlinkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("radio error: {0}")]
    Radio(#[from] sidebridge::RadioError),

    #[error("WebRTC error: {0}")]
    WebRtc(String),

    #[error("audio error: {0}")]
    Audio(String),
}

pub type Result<T> = std::result::Result<T, CivlinkError>;
