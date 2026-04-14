use num_complex::Complex32;
use std::f32::consts::PI;

pub struct ComplexRotator {
    phase: Complex32,
    step: Complex32,
    counter: usize,
    reset_interval: usize,
}

impl ComplexRotator {
    /// Create a new rotator for a given frequency offset (Hz) and sample rate (Hz)
    pub fn new(freq_shift_hz: f32, sample_rate: f32, reset_interval: usize) -> Self {
        let omega = 2.0 * PI * freq_shift_hz / sample_rate;

        ComplexRotator {
            phase: Complex32::new(1.0, 0.0),
            step: Complex32::from_polar(1.0, omega), // exp(jω)
            counter: 0,
            reset_interval,
        }
    }

    /// Apply rotation to a single complex sample
    pub fn rotate(&mut self, input: Complex32) -> Complex32 {
        let output = input * self.phase;
        self.phase *= self.step;

        self.counter += 1;
        if self.counter >= self.reset_interval {
            self.phase /= self.phase.norm();
            self.counter = 0;
        }

        output
    }

    /// Reset the rotator phase (optional external control)
    pub fn reset(&mut self) {
        self.phase = Complex32::new(1.0, 0.0);
        self.counter = 0;
    }

    /// Change the shift frequency without disturbing the running phase, so
    /// retuning a live stream is click-free.
    pub fn set_freq(&mut self, freq_shift_hz: f32, sample_rate: f32) {
        let omega = 2.0 * PI * freq_shift_hz / sample_rate;
        self.step = Complex32::from_polar(1.0, omega);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex32;
    use signals::ComplexOscillator;

    fn approx_eq(a: Complex32, b: Complex32, tol: f32) -> bool {
        (a - b).norm() < tol
    }

    #[test]
    fn test_rotator_shifts_frequency_correctly() {
        // Test over a short window where cumulative drift between two
        // independent oscillators hasn't diverged significantly.
        let len = 2048;
        let sample_rate = 768_000.0;
        let freq_0 = 1_000.0;
        let freq_1 = -5_000.0;
        let delta_f = freq_1 - freq_0;

        // Input signal
        let osc0 = ComplexOscillator::new(freq_0, sample_rate);
        let input = osc0.take(len).collect::<Vec<_>>();

        // Expected output signal after frequency shift
        let osc1 = ComplexOscillator::new(freq_1, sample_rate);
        let expected: Vec<Complex32> = osc1.take(len).collect();

        // Rotator to shift f0 → f1
        let mut rot = ComplexRotator::new(delta_f, sample_rate, 512);

        let rotated: Vec<Complex32> = input.iter().map(|&x| rot.rotate(x)).collect();

        for (i, (r, e)) in rotated.iter().zip(expected.iter()).enumerate() {
            let ratio = r / e;
            let phase_error = ratio.arg();
            assert!(
                phase_error.abs() < 1e-3,
                "Phase error too large at sample {i}: rotated = {r}, expected = {e}, phase_error = {phase_error}"
            );
        }
    }

    #[test]
    fn test_rotator_preserves_magnitude() {
        // Over a long run, the rotator should preserve signal magnitude
        let len = 768_000;
        let sample_rate = 768_000.0;
        let mut rot = ComplexRotator::new(-5000.0, sample_rate, 512);

        let osc = ComplexOscillator::new(1000.0, sample_rate);
        let input = osc.take(len).collect::<Vec<_>>();

        for (i, &x) in input.iter().enumerate() {
            let y = rot.rotate(x);
            assert!(
                (y.norm() - x.norm()).abs() < 1e-4,
                "Magnitude changed at sample {i}: input = {}, output = {}",
                x.norm(),
                y.norm()
            );
        }
    }

    #[test]
    fn test_phase_normalization_over_time() {
        let mut rot = ComplexRotator::new(5000.0, 48000.0, 512);

        for _ in 0..10_000 {
            rot.rotate(Complex32::new(1.0, 0.0));
        }

        let norm = rot.phase.norm();
        assert!(
            (norm - 1.0).abs() < 1e-5,
            "Phase norm drifted too far from 1.0: norm = {norm}"
        );
    }

    #[test]
    fn test_reset_behavior() {
        let mut rot = ComplexRotator::new(1000.0, 48000.0, 512);

        rot.rotate(Complex32::new(1.0, 0.0));
        rot.rotate(Complex32::new(1.0, 0.0));
        rot.reset();

        assert!(
            approx_eq(rot.phase, Complex32::new(1.0, 0.0), 1e-6),
            "Reset did not restore phase correctly"
        );
    }
}
