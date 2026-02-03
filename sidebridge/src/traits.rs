use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tokio_stream::Stream;

/// Specific errors that can occur within the sidebridge ecosystem.
/// Using thiserror allows library consumers to handle hardware issues programmatically.
#[derive(Error, Debug)]
pub enum RadioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hardware timeout: radio at {0} did not respond")]
    Timeout(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("The radio is currently busy or in a state that prevents this action")]
    Busy,

    #[error("Feature not supported by this hardware: {0}")]
    Unsupported(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// A specialized Result type for sidebridge operations.
pub type Result<T> = std::result::Result<T, RadioError>;

/// Core operating modes for amateur radio.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RadioMode {
    LSB,
    USB,
    AM,
    FM,
    CW,
    RTTY,
    CWR,
    RTTYR,
    DataLSB,
    DataUSB,
    DataFM,
}

impl fmt::Display for RadioMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// The snapshot of the radio's current configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadioState {
    pub frequency_hz: u64,
    pub mode: RadioMode,
    pub ptt_active: bool,
    pub rf_power_percent: u8,
}

/// Spectrum data frame for waterfall rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumFrame {
    pub center_freq_hz: u64,
    pub span_hz: u32,
    /// Amplitude bins (typically 0-255).
    pub bins: Vec<u8>,
}

/// The core interface that all radio drivers must implement.
/// Following the `cpal` style, we name this with the `Trait` suffix.
#[async_trait]
pub trait RadioTrait: Send + Sync {
    /// A unique identifier for the specific radio instance.
    fn id(&self) -> &str;

    // --- Commands (The "Setters") ---

    /// Set the VFO frequency.
    async fn set_frequency(&self, hz: u64) -> Result<()>;

    /// Set the radio's operating mode.
    async fn set_mode(&self, mode: RadioMode) -> Result<()>;

    /// Trigger PTT (Transmit/Receive).
    async fn set_ptt(&self, active: bool) -> Result<()>;

    // --- Telemetry (The "Objective-C" style Getters) ---

    /// Retrieve the current state of the radio hardware.
    async fn state(&self) -> Result<RadioState>;

    /// A stream that emits new `RadioState` whenever hardware values change.
    fn state_updates(&self) -> Box<dyn Stream<Item = RadioState> + Send + Unpin>;

    /// An optional stream for spectrum scope/waterfall data.
    fn spectrum(&self) -> Option<Box<dyn Stream<Item = SpectrumFrame> + Send + Unpin>> {
        None
    }
}
