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
    WebRtc(#[from] webrtc::Error),

    #[error("audio device enumeration: {0}")]
    AudioDevices(#[from] cpal::DevicesError),

    #[error("audio default config: {0}")]
    AudioDefaultConfig(#[from] cpal::DefaultStreamConfigError),

    #[error("audio supported configs: {0}")]
    AudioSupportedConfigs(#[from] cpal::SupportedStreamConfigsError),

    #[error("audio build stream: {0}")]
    AudioBuildStream(#[from] cpal::BuildStreamError),

    #[error("audio play stream: {0}")]
    AudioPlayStream(#[from] cpal::PlayStreamError),

    #[error("opus: {0}")]
    Opus(#[from] opus::Error),

    #[error("audio error: {0}")]
    Audio(String),
}

pub type Result<T> = std::result::Result<T, CivlinkError>;
