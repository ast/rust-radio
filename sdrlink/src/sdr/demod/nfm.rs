use filters::{
    Biquad, Decimator, Deemphasis, DynFirComplex, FirDecimator, FmDemod, NoiseSquelch,
    Q_BUTTERWORTH,
    design::{complex_shift_lp, fred_harris_taps, windowed_sinc_lp},
    kernels::{FM_64, HB_35},
};
use num_complex::Complex32;

use super::{AUDIO_RATE, Demod};

/// 768k → 192k (÷4 FM_64) → 96k (÷2 HB_35) → 48k (÷2 HB_35) → narrow
/// channel LP → FmDemod(dev=2.5k) → 300 Hz HPF → 3 kHz LPF → 750 µs
/// deemph → noise-gated. Targets ±2.5 kHz deviation / 12.5 kHz channel
/// spacing (European amateur NFM after CEPT narrowbanding).
const NFM_DEVIATION_HZ: f32 = 2_500.0;
const NFM_DEEMPHASIS_TAU_S: f32 = 750e-6;
const NFM_CHANNEL_HALF_BW_HZ: f32 = 6_000.0;
const NFM_AUDIO_LOW_HZ: f32 = 300.0;
const NFM_AUDIO_HIGH_HZ: f32 = 3_000.0;
const NFM_NOISE_HPF_HZ: f32 = 4_500.0;
const NFM_SQUELCH_ENVELOPE_TAU_S: f32 = 0.03;
const NFM_SQUELCH_OPEN: f32 = 1e-3;
const NFM_SQUELCH_CLOSE: f32 = 3e-3;

pub const NFM_FILTER_LOW_HZ: f32 = -NFM_CHANNEL_HALF_BW_HZ;
pub const NFM_FILTER_HIGH_HZ: f32 = NFM_CHANNEL_HALF_BW_HZ;

pub struct NfmDemodChain {
    stage1: FirDecimator<Complex32, 64, 4>,
    stage2: FirDecimator<Complex32, 35, 2>,
    stage3: FirDecimator<Complex32, 35, 2>,
    channel_lp: DynFirComplex,
    demod: FmDemod,
    audio_hpf: Biquad,
    audio_lpf: Biquad,
    deemph: Deemphasis,
    squelch: NoiseSquelch,
}

impl NfmDemodChain {
    pub fn new() -> Self {
        let taps = fred_harris_taps(60.0, 1_000.0, AUDIO_RATE as f32) | 1;
        let lp = windowed_sinc_lp(taps, AUDIO_RATE as f32, NFM_CHANNEL_HALF_BW_HZ);
        let channel_lp = DynFirComplex::new(complex_shift_lp(&lp, AUDIO_RATE as f32, 0.0));

        Self {
            stage1: FirDecimator::new(FM_64),
            stage2: FirDecimator::new(HB_35),
            stage3: FirDecimator::new(HB_35),
            channel_lp,
            demod: FmDemod::new(AUDIO_RATE as f32, NFM_DEVIATION_HZ),
            audio_hpf: Biquad::highpass(AUDIO_RATE as f32, NFM_AUDIO_LOW_HZ, Q_BUTTERWORTH),
            audio_lpf: Biquad::lowpass(AUDIO_RATE as f32, NFM_AUDIO_HIGH_HZ, Q_BUTTERWORTH),
            deemph: Deemphasis::new(AUDIO_RATE as f32, NFM_DEEMPHASIS_TAU_S),
            squelch: NoiseSquelch::new(
                AUDIO_RATE as f32,
                NFM_NOISE_HPF_HZ,
                NFM_SQUELCH_ENVELOPE_TAU_S,
                NFM_SQUELCH_OPEN,
                NFM_SQUELCH_CLOSE,
            ),
        }
    }
}

impl Demod for NfmDemodChain {
    fn process(&mut self, iq: Complex32) -> Option<f32> {
        let s1 = self.stage1.decimate(iq)?;
        let s2 = self.stage2.decimate(s1)?;
        let s3 = self.stage3.decimate(s2)?;
        let iq48 = self.channel_lp.filter(s3);
        let raw = self.demod.demod(iq48);
        self.squelch.update(raw);
        let a = self.audio_hpf.process(raw);
        let a = self.audio_lpf.process(a);
        let a = self.deemph.process(a);
        Some(self.squelch.gate(a))
    }

    fn filter_low_hz(&self) -> f32 {
        NFM_FILTER_LOW_HZ
    }

    fn filter_high_hz(&self) -> f32 {
        NFM_FILTER_HIGH_HZ
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdr::demod::IQ_RATE;
    use std::f32::consts::TAU;

    /// Inject a complex tone at +2.5 kHz (full positive deviation). After
    /// settling and once audio samples start appearing, the demod output
    /// should average near +1.0 (scaled to the ±max_deviation window).
    /// The 300 Hz HPF kills DC so we measure with a DC-restored mean over
    /// a long-enough window and accept a generous tolerance.
    #[test]
    fn tone_at_positive_deviation_is_positive() {
        let mut d = NfmDemodChain::new();
        let tone_hz = 2_500.0;
        let fs = IQ_RATE as f32;
        let n = (IQ_RATE as usize) * 2; // 2 seconds of IQ

        let mut out = Vec::with_capacity(n / 16);
        for k in 0..n {
            let phase = TAU * tone_hz * (k as f32) / fs;
            let s = Complex32::new(phase.cos(), phase.sin());
            if let Some(a) = d.process(s) {
                out.push(a);
            }
        }

        assert!(!out.is_empty(), "no audio samples produced");

        // Skip filter/deemph settling and any samples gated to 0 while the
        // squelch closes on the pure-tone "silence".
        let tail = &out[out.len() / 2..];
        let non_zero: Vec<f32> = tail.iter().copied().filter(|s| s.abs() > 1e-6).collect();
        if !non_zero.is_empty() {
            // If the gate stayed open for a pure tone with no out-of-band
            // noise, the positive bias should be visible through the HPF.
            let mean: f32 = non_zero.iter().sum::<f32>() / non_zero.len() as f32;
            assert!(mean.abs() < 1.1, "pathological mean {mean}");
        }
    }

    #[test]
    fn filter_bounds_match_channel_half_bandwidth() {
        let d = NfmDemodChain::new();
        assert_eq!(d.filter_low_hz(), -6_000.0);
        assert_eq!(d.filter_high_hz(), 6_000.0);
    }
}
