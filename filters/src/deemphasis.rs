/// 1-pole IIR de-emphasis lowpass for broadcast FM audio.
/// τ = 50 µs (ITU R1, incl. Europe), 75 µs (North America/Korea).
pub struct Deemphasis {
    alpha: f32,
    y: f32,
}

impl Deemphasis {
    pub fn new(sample_rate_hz: f32, tau_seconds: f32) -> Self {
        let dt = 1.0 / sample_rate_hz;
        let alpha = dt / (tau_seconds + dt);
        Self { alpha, y: 0.0 }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        self.y += self.alpha * (x - self.y);
        self.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    fn magnitude_db(fs: f32, tau: f32, freq: f32) -> f32 {
        let mut d = Deemphasis::new(fs, tau);
        // Settle.
        for _ in 0..2000 {
            d.process(0.0);
        }
        let n = 4000;
        let mut acc_re = 0.0f32;
        let mut acc_im = 0.0f32;
        for k in 0..n {
            let phase = TAU * freq * (k as f32) / fs;
            let y = d.process(phase.cos());
            acc_re += y * phase.cos();
            acc_im += y * phase.sin();
        }
        // x = cos has unit-amplitude single tone; coherent detect → 0.5 * H(f).
        let mag = 2.0 * (acc_re * acc_re + acc_im * acc_im).sqrt() / n as f32;
        20.0 * mag.log10()
    }

    #[test]
    fn dc_gain_unity() {
        let mut d = Deemphasis::new(48_000.0, 50e-6);
        for _ in 0..10_000 {
            d.process(1.0);
        }
        let y = d.process(1.0);
        assert!((y - 1.0).abs() < 1e-4, "settled DC = {y}");
    }

    #[test]
    fn cutoff_minus_three_db() {
        let fs = 48_000.0;
        let tau = 50e-6;
        let fc = 1.0 / (TAU * tau); // ≈ 3183 Hz
        let db = magnitude_db(fs, tau, fc);
        // Discrete 1-pole IIR has slightly more attenuation than the ideal
        // analog -3 dB at the analog cutoff; allow ~1 dB extra droop.
        assert!(db < -2.5 && db > -4.5, "|H(fc)| = {db} dB");
    }

    #[test]
    fn fifteen_khz_attenuation() {
        let fs = 48_000.0;
        let tau = 50e-6;
        let db = magnitude_db(fs, tau, 15_000.0);
        // Analog 1-pole at 15 kHz with fc=3183 Hz: -20*log10(sqrt(1+(15/3.183)^2)) ≈ -13.7 dB.
        // Bilinear-ish IIR is close; allow 1.5 dB tolerance.
        assert!(db < -12.0 && db > -16.0, "|H(15k)| = {db} dB");
    }
}
