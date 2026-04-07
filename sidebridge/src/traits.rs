// Copyright SM6WJM 2026

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tokio_stream::Stream;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum RadioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Radio busy")]
    Busy,

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Command rejected by radio")]
    CommandFailed,
}

pub type Result<T> = std::result::Result<T, RadioError>;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Operating mode — covers modes common across Icom, Yaesu, and Kenwood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Lsb,
    Usb,
    Am,
    Cw,
    Rtty,
    Fm,
    CwR,
    RttyR,
    DataLsb,
    DataUsb,
    DataFm,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::Lsb => write!(f, "LSB"),
            Mode::Usb => write!(f, "USB"),
            Mode::Am => write!(f, "AM"),
            Mode::Cw => write!(f, "CW"),
            Mode::Rtty => write!(f, "RTTY"),
            Mode::Fm => write!(f, "FM"),
            Mode::CwR => write!(f, "CW-R"),
            Mode::RttyR => write!(f, "RTTY-R"),
            Mode::DataLsb => write!(f, "DATA-LSB"),
            Mode::DataUsb => write!(f, "DATA-USB"),
            Mode::DataFm => write!(f, "DATA-FM"),
        }
    }
}

/// Runtime capability discovery.
#[derive(Debug, Clone)]
pub struct Capabilities {
    pub meter: bool,
    pub scope: bool,
    pub swr: bool,
    pub memory_channels: bool,
}

/// Spectrum scope frame for waterfall rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeFrame {
    pub center_hz: u64,
    pub span_hz: u32,
    /// Amplitude bins (typically 0–255).
    pub bins: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Core trait — every radio implements this
// ---------------------------------------------------------------------------

/// Core radio interface: frequency, mode, and PTT.
///
/// This is the minimum every radio must implement, from a modern IC-705
/// to a vintage IC-781.
#[async_trait]
pub trait Radio: Send + Sync {
    /// Unique identifier for this radio instance.
    fn id(&self) -> &str;

    async fn set_frequency(&self, hz: u64) -> Result<()>;
    async fn frequency(&self) -> Result<u64>;

    async fn set_mode(&self, mode: Mode) -> Result<()>;
    async fn mode(&self) -> Result<Mode>;

    async fn set_ptt(&self, active: bool) -> Result<()>;
    async fn ptt(&self) -> Result<bool>;
}

// ---------------------------------------------------------------------------
// Capability traits — implement what the hardware supports
// ---------------------------------------------------------------------------

/// Static information about the radio and its capabilities.
#[async_trait]
pub trait RadioInfo: Radio {
    /// Human-readable model name (e.g. "IC-705", "FT-891").
    fn model(&self) -> &str;

    /// Which optional capabilities this radio supports.
    fn capabilities(&self) -> Capabilities;
}

/// Metering: S-meter, SWR, ALC, RF power.
#[async_trait]
pub trait RadioMeter: Radio {
    /// S-meter reading (0–255 raw, mapping is radio-specific).
    async fn signal_strength(&self) -> Result<u8>;

    /// Standing wave ratio (None if not supported or not transmitting).
    async fn swr(&self) -> Result<Option<f32>>;

    /// Automatic level control (None if not supported or not transmitting).
    async fn alc(&self) -> Result<Option<f32>>;

    /// Forward RF power in watts (None if not supported or not transmitting).
    async fn rf_power(&self) -> Result<Option<f32>>;
}

/// Spectrum scope / waterfall (e.g. IC-705 bandscope).
#[async_trait]
pub trait RadioScope: Radio {
    /// Request a single scope frame.
    async fn scope_data(&self) -> Result<ScopeFrame>;

    /// Continuous stream of scope frames.
    fn scope_stream(&self) -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin>;
}
