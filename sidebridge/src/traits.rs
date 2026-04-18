// Copyright SM6WJM 2026

use arrayvec::ArrayVec;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use tokio::sync::mpsc;

/// Maximum number of amplitude bins in a scope frame.
pub const MAX_SCOPE_BINS: usize = 512;

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

/// Frequency information for a scope frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum ScopeFreq {
    /// Center mode: tuned frequency at center, symmetric span.
    #[serde(rename = "center")]
    Center { center_hz: u64, span_hz: u64 },
    /// Fixed mode: absolute lower and upper edge frequencies.
    #[serde(rename = "fixed")]
    Fixed { lower_hz: u64, upper_hz: u64 },
}

/// Scope display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeMode {
    /// Operating frequency is always at the screen center.
    Center,
    /// Display spans a fixed frequency range selected from the radio's
    /// stored edge presets (1–3 on IC-705 / IC-7300).
    Fixed,
}

/// Spectrum scope frame for waterfall rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeFrame {
    pub freq: ScopeFreq,
    /// Amplitude bins (typically 0–160 on Icom radios).
    pub bins: ArrayVec<u8, MAX_SCOPE_BINS>,
}

/// Radio event — frequency changes, mode changes, scope data, and meter
/// readings all arrive over the same link and are delivered through a
/// single stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum RadioEvent {
    Frequency(u64),
    Mode(Mode, u8),
    Scope(ScopeFrame),
    SignalMeter(u8),
    RfPower(u8),
    Swr(u8),
    Alc(u8),
    /// RF gain level, 0–255 (Icom raw scale).
    RfGain(u8),
}

/// Radio command — user-initiated actions sent from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum RadioCommand {
    SetFrequency(u64),
    SetMode(Mode),
    SetScopeSpan(u64),
    SetScopeMode(ScopeMode),
    /// Select one of the radio's stored fixed-edge presets for the current band (1–3).
    SetScopeFixedEdge(u8),
    /// RF gain (0–255).
    SetRfGain(u8),
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
    /// Request a frequency reading. Result arrives as `RadioEvent::Frequency`.
    async fn read_frequency(&self) -> Result<()>;

    async fn set_mode(&self, mode: Mode) -> Result<()>;
    /// Request a mode reading. Result arrives as `RadioEvent::Mode`.
    async fn read_mode(&self) -> Result<()>;

    async fn set_ptt(&self, active: bool) -> Result<()>;
    async fn ptt(&self) -> Result<bool>;

    /// Take the event stream receiver. Returns `None` if already taken.
    ///
    /// This is a single-consumer stream — call it once and fan out in the
    /// application layer if multiple consumers are needed.
    fn take_event_stream(&self) -> Option<mpsc::Receiver<RadioEvent>>;
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
///
/// Each method sends the read command to the radio. The result arrives
/// asynchronously as a `RadioEvent` on the event stream.
#[async_trait]
pub trait RadioMeter: Radio {
    /// Request an S-meter reading. Result arrives as `RadioEvent::SignalMeter`.
    async fn read_signal_strength(&self) -> Result<()>;

    /// Request a SWR reading. Result arrives as `RadioEvent::Swr`.
    async fn read_swr(&self) -> Result<()>;

    /// Request an ALC reading. Result arrives as `RadioEvent::Alc`.
    async fn read_alc(&self) -> Result<()>;

    /// Request an RF power reading. Result arrives as `RadioEvent::RfPower`.
    async fn read_rf_power(&self) -> Result<()>;
}

/// Spectrum scope / waterfall (e.g. IC-705 bandscope).
#[async_trait]
pub trait RadioScope: Radio {
    /// Enable or disable scope waveform output on the radio.
    async fn set_scope_output(&self, enable: bool) -> Result<()>;

    /// Set the scope span in center mode.
    /// Valid values: 2500, 5000, 10000, 25000, 50000, 100000, 250000, 500000 Hz.
    async fn set_scope_span(&self, span_hz: u64) -> Result<()>;

    /// Switch the scope between Center and Fixed display modes.
    async fn set_scope_mode(&self, mode: ScopeMode) -> Result<()>;

    /// Select one of the radio's stored Fixed-mode edge presets (1, 2, or 3)
    /// for the band the VFO is currently tuned to.
    async fn set_scope_fixed_edge(&self, edge: u8) -> Result<()>;
}

/// Gain controls (RF gain, and potentially AF/mic gain in the future).
#[async_trait]
pub trait RadioGain: Radio {
    /// Set RF gain level (0–255 on Icom radios).
    async fn set_rf_gain(&self, value: u8) -> Result<()>;

    /// Request the current RF gain. Result arrives as `RadioEvent::RfGain`.
    async fn read_rf_gain(&self) -> Result<()>;
}
