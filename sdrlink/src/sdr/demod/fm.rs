use filters::{
    Decimator, Deemphasis, FirDecimator, FmDemod,
    kernels::{FM_64, HB_35},
};
use num_complex::Complex32;

use super::{AUDIO_RATE, Demod, IQ_RATE};

/// 768k → 192k (÷4 FM_64) → FmDemod → 96k → 48k (÷2 HB_35 ×2) → deemph.
const FM_DEVIATION_HZ: f32 = 75_000.0;
const FM_DEEMPHASIS_TAU_S: f32 = 50e-6;
pub const FM_FILTER_LOW_HZ: f32 = -100_000.0;
pub const FM_FILTER_HIGH_HZ: f32 = 100_000.0;

const DEMOD_RATE: u32 = IQ_RATE / 4;

pub struct FmDemodChain {
    stage1: FirDecimator<Complex32, 64, 4>,
    demod: FmDemod,
    stage2: FirDecimator<f32, 35, 2>,
    stage3: FirDecimator<f32, 35, 2>,
    deemph: Deemphasis,
}

impl FmDemodChain {
    pub fn new() -> Self {
        Self {
            stage1: FirDecimator::new(FM_64),
            demod: FmDemod::new(DEMOD_RATE as f32, FM_DEVIATION_HZ),
            stage2: FirDecimator::new(HB_35),
            stage3: FirDecimator::new(HB_35),
            deemph: Deemphasis::new(AUDIO_RATE as f32, FM_DEEMPHASIS_TAU_S),
        }
    }
}

impl Demod for FmDemodChain {
    fn process(&mut self, iq: Complex32) -> Option<f32> {
        let iq_dec = self.stage1.decimate(iq)?;
        let audio = self.demod.demod(iq_dec);
        let a1 = self.stage2.decimate(audio)?;
        let a2 = self.stage3.decimate(a1)?;
        Some(self.deemph.process(a2))
    }

    fn filter_low_hz(&self) -> f32 {
        FM_FILTER_LOW_HZ
    }

    fn filter_high_hz(&self) -> f32 {
        FM_FILTER_HIGH_HZ
    }
}
