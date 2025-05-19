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
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_complex::Complex32;

    fn approx_eq(a: Complex32, b: Complex32, tol: f32) -> bool {
        (a - b).norm() < tol
    }

    fn generate_complex_input(len: usize, sample_rate: f32, freq_hz: f32) -> Vec<Complex32> {
        let omega = 2.0 * PI * freq_hz / sample_rate;

        (0..len)
            .map(|i| {
                let phase = omega * i as f32;
                let (im, re) = phase.sin_cos();
                Complex32::new(re, im) // cos + j·sin = e^(j·phase)
            })
            .collect()
    }

    #[test]
    fn test_rotator_shifts_frequency_correctly() {
        let len = 768_000;
        let sample_rate = 768_000.0;
        let f1 = 1_000.0;
        let f2 = -5_000.0;
        let delta_f = f2 - f1;

        // Original signal at f1
        let input = generate_complex_input(len, sample_rate, f1);
        // Expected signal at f2
        let expected = generate_complex_input(len, sample_rate, f2);

        // Rotator to shift f1 → f2
        let mut rot = ComplexRotator::new(delta_f, sample_rate, 512);

        let rotated: Vec<Complex32> = input.iter().map(|&x| rot.rotate(x)).collect();

        for (i, (r, e)) in rotated.iter().zip(expected.iter()).enumerate() {
            let ratio = r / e;
            let phase_error = ratio.arg(); // should be ≈ 0 if r ≈ e (modulo amplitude)
            assert!(
            phase_error.abs() < 5e-3,
            "Phase error too large at sample {i}: rotated = {r}, expected = {e}, phase_error = {phase_error}"
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
