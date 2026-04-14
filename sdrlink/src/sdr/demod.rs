pub mod fm;
pub mod ssb;

use num_complex::Complex32;
use serde::{Deserialize, Serialize};

pub use fm::FmDemodChain;
pub use ssb::SsbDemodChain;

/// All demods take rotated IQ at the capture rate (768k) and emit audio
/// samples at [`AUDIO_RATE`] (48k). Sample-by-sample so the pipeline can
/// keep the demod behind a `Box<dyn Demod>` without buffer plumbing.
pub trait Demod: Send {
    fn process(&mut self, iq: Complex32) -> Option<f32>;
    fn filter_low_hz(&self) -> f32;
    fn filter_high_hz(&self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Fm,
    Usb,
    Lsb,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Fm => "fm",
            Mode::Usb => "usb",
            Mode::Lsb => "lsb",
        }
    }
}

/// Sample rates for the shared multistage decimation chain.
/// 768k → 192k (÷4) → 48k (÷4). Chosen per the multistage analysis in
/// `filters/docs/Close to optimal FIR decimator design.pdf`: wide early
/// transition bands keep the high-rate stages cheap.
pub const IQ_RATE: u32 = 768_000;
pub const AUDIO_RATE: u32 = 48_000;

pub fn build(mode: Mode) -> Box<dyn Demod> {
    match mode {
        Mode::Fm => Box::new(FmDemodChain::new()),
        Mode::Usb => Box::new(SsbDemodChain::new(filters::design::SsbSide::Upper, 2_700.0)),
        Mode::Lsb => Box::new(SsbDemodChain::new(filters::design::SsbSide::Lower, 2_700.0)),
    }
}

/// Passband `(low_hz, high_hz)` a freshly-built demod for `mode` would
/// report. Lets signaling respond to a mode change without waiting for
/// the worker thread to rebuild.
pub fn filter_hz_for(mode: Mode) -> (f32, f32) {
    let d = build(mode);
    (d.filter_low_hz(), d.filter_high_hz())
}
