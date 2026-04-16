use std::f32::consts::TAU;

/// Butterworth Q for a single biquad section (2-pole, -3 dB at fc).
pub const Q_BUTTERWORTH: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// Second-order IIR (biquad) in Direct Form II Transposed. Coefficients from
/// Robert Bristow-Johnson's audio EQ cookbook.
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    pub fn lowpass(sample_rate_hz: f32, cutoff_hz: f32, q: f32) -> Self {
        let w0 = TAU * cutoff_hz / sample_rate_hz;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) * 0.5;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::from_coeffs(b0, b1, b2, a0, a1, a2)
    }

    pub fn highpass(sample_rate_hz: f32, cutoff_hz: f32, q: f32) -> Self {
        let w0 = TAU * cutoff_hz / sample_rate_hz;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 + cos_w0) * 0.5;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::from_coeffs(b0, b1, b2, a0, a1, a2)
    }

    fn from_coeffs(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn magnitude_db(mut bq: Biquad, fs: f32, freq: f32) -> f32 {
        for _ in 0..4000 {
            bq.process(0.0);
        }
        let n = 8000;
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for k in 0..n {
            let phase = TAU * freq * (k as f32) / fs;
            let y = bq.process(phase.cos());
            re += y * phase.cos();
            im += y * phase.sin();
        }
        let mag = 2.0 * (re * re + im * im).sqrt() / n as f32;
        20.0 * mag.log10()
    }

    #[test]
    fn lowpass_dc_unity() {
        let db = magnitude_db(Biquad::lowpass(48_000.0, 3_000.0, Q_BUTTERWORTH), 48_000.0, 50.0);
        assert!(db.abs() < 0.2, "DC gain = {db} dB");
    }

    #[test]
    fn lowpass_minus_three_db_at_cutoff() {
        let db = magnitude_db(Biquad::lowpass(48_000.0, 3_000.0, Q_BUTTERWORTH), 48_000.0, 3_000.0);
        assert!(db < -2.5 && db > -3.5, "|H(fc)| = {db} dB");
    }

    #[test]
    fn lowpass_deep_stopband() {
        let db = magnitude_db(Biquad::lowpass(48_000.0, 3_000.0, Q_BUTTERWORTH), 48_000.0, 12_000.0);
        // 2-pole Butterworth: -12 dB/oct → 2 octaves above fc ≈ -24 dB.
        assert!(db < -20.0, "|H(4·fc)| = {db} dB");
    }

    #[test]
    fn highpass_dc_zero() {
        let db = magnitude_db(Biquad::highpass(48_000.0, 300.0, Q_BUTTERWORTH), 48_000.0, 10.0);
        assert!(db < -20.0, "DC gain = {db} dB");
    }

    #[test]
    fn highpass_passband_unity() {
        let db = magnitude_db(Biquad::highpass(48_000.0, 300.0, Q_BUTTERWORTH), 48_000.0, 3_000.0);
        assert!(db.abs() < 0.5, "|H(10·fc)| = {db} dB");
    }

    #[test]
    fn highpass_minus_three_db_at_cutoff() {
        let db = magnitude_db(Biquad::highpass(48_000.0, 300.0, Q_BUTTERWORTH), 48_000.0, 300.0);
        assert!(db < -2.5 && db > -3.5, "|H(fc)| = {db} dB");
    }
}
