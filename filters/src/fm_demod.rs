use num_complex::Complex32;

/// Quadrature FM demodulator: `out = arg(conj(prev) * curr)`, scaled so that
/// a tone at `+max_deviation_hz` maps to +1.0 and `-max_deviation_hz` to -1.0.
pub struct FmDemod {
    prev: Complex32,
    scale: f32,
}

impl FmDemod {
    /// `sample_rate_hz` is the IQ rate feeding the demod; `max_deviation_hz` is
    /// the nominal peak deviation (75 kHz for broadcast FM).
    pub fn new(sample_rate_hz: f32, max_deviation_hz: f32) -> Self {
        let scale = sample_rate_hz / (2.0 * std::f32::consts::PI * max_deviation_hz);
        Self { prev: Complex32::new(1.0, 0.0), scale }
    }

    pub fn demod(&mut self, sample: Complex32) -> f32 {
        let p = self.prev.conj() * sample;
        self.prev = sample;
        p.im.atan2(p.re) * self.scale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn tone_at_max_deviation_reads_one() {
        let fs = 192_000.0;
        let dev = 75_000.0;
        let tone = 75_000.0;
        let mut d = FmDemod::new(fs, dev);
        // Skip the first sample (prev is seed).
        d.demod(Complex32::new(1.0, 0.0));
        let mut sum = 0.0;
        let n = 1000;
        for k in 1..=n {
            let phase = TAU * tone * (k as f32) / fs;
            let s = Complex32::new(phase.cos(), phase.sin());
            sum += d.demod(s);
        }
        let avg = sum / n as f32;
        assert!((avg - 1.0).abs() < 1e-4, "avg={avg}");
    }

    #[test]
    fn tone_at_negative_half_deviation() {
        let fs = 192_000.0;
        let dev = 75_000.0;
        let tone = -37_500.0;
        let mut d = FmDemod::new(fs, dev);
        d.demod(Complex32::new(1.0, 0.0));
        let mut sum = 0.0;
        let n = 1000;
        for k in 1..=n {
            let phase = TAU * tone * (k as f32) / fs;
            let s = Complex32::new(phase.cos(), phase.sin());
            sum += d.demod(s);
        }
        let avg = sum / n as f32;
        assert!((avg + 0.5).abs() < 1e-4, "avg={avg}");
    }

    #[test]
    fn dc_reads_zero() {
        let mut d = FmDemod::new(192_000.0, 75_000.0);
        d.demod(Complex32::new(0.7, 0.3));
        for _ in 0..10 {
            let out = d.demod(Complex32::new(0.7, 0.3));
            assert!(out.abs() < 1e-6);
        }
    }
}
