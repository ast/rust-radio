use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdrLinkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("SDR error: {0}")]
    Sdr(String),

    #[error("WebRTC error: {0}")]
    WebRtc(String),
}

pub type Result<T> = std::result::Result<T, SdrLinkError>;
