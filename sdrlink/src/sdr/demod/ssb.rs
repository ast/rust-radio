use filters::{
    Decimator, DynFirComplex, FirDecimator,
    design::{SsbSide, fred_harris_taps, ssb_bandpass},
    kernels::{FM_64, HB_35},
};
use num_complex::Complex32;

use super::{AUDIO_RATE, Demod};

/// SSB: stay complex through the decimation chain, then apply an asymmetric
/// bandpass at audio rate and take the real part.
///
/// 768k → 192k (÷4 FM_64) → 96k (÷2 HB_35) → 48k (÷2 HB_35) → complex
/// bandpass (windowed-sinc, Fred Harris sized) → .re.
///
/// Reusing FM_64 for the first stage is wasteful for SSB (its transition
/// band is much wider than SSB needs) but keeps things simple and cheap
/// in MADS at the high-rate stage per the paper's guidance that early
/// stages dominate cost only when their tap count is comparable to
/// later stages — here 64 taps at 768k still beats redesigning.
pub struct SsbDemodChain {
    stage1: FirDecimator<Complex32, 64, 4>,
    stage2: FirDecimator<Complex32, 35, 2>,
    stage3: FirDecimator<Complex32, 35, 2>,
    bp: DynFirComplex,
    side: SsbSide,
    width_hz: f32,
}

impl SsbDemodChain {
    pub fn new(side: SsbSide, width_hz: f32) -> Self {
        // 60 dB stop, 500 Hz transition at 48 kHz → ≈262 taps.
        let taps = fred_harris_taps(60.0, 500.0, AUDIO_RATE as f32) | 1;
        let bp = DynFirComplex::new(ssb_bandpass(taps, AUDIO_RATE as f32, width_hz, side));

        Self {
            stage1: FirDecimator::new(FM_64),
            stage2: FirDecimator::new(HB_35),
            stage3: FirDecimator::new(HB_35),
            bp,
            side,
            width_hz,
        }
    }
}

impl Demod for SsbDemodChain {
    fn process(&mut self, iq: Complex32) -> Option<f32> {
        let s1 = self.stage1.decimate(iq)?;
        let s2 = self.stage2.decimate(s1)?;
        let s3 = self.stage3.decimate(s2)?;
        Some(self.bp.filter(s3).re)
    }

    fn filter_low_hz(&self) -> f32 {
        match self.side {
            SsbSide::Upper => 0.0,
            SsbSide::Lower => -self.width_hz,
        }
    }

    fn filter_high_hz(&self) -> f32 {
        match self.side {
            SsbSide::Upper => self.width_hz,
            SsbSide::Lower => 0.0,
        }
    }
}

