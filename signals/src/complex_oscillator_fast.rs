use num_complex::Complex32;
use std::f32::consts::PI;

pub struct ComplexOscillatorFast {
    phase: Complex32,
    step: Complex32,
    counter: usize,
    reset_interval: usize,
}

impl ComplexOscillatorFast {
    /// Create a new oscillator for a given frequency (Hz) and sample rate (Hz)
    pub fn new(frequency: f32, sample_rate: f32, reset_interval: usize) -> Self {
        let omega = 2.0 * PI * frequency / sample_rate;

        Self {
            phase: Complex32::new(1.0, 0.0),
            step: Complex32::from_polar(1.0, omega), // exp(jω)
            counter: 0,
            reset_interval,
        }
    }

    /// Reset the rotator phase (optional external control)
    pub fn reset(&mut self) {
        self.phase = Complex32::new(1.0, 0.0);
        self.counter = 0;
    }
}

impl Iterator for ComplexOscillatorFast {
    type Item = Complex32;

    fn next(&mut self) -> Option<Self::Item> {
        // Rotate and return the complex value
        let result = self.phase;
        self.phase *= self.step;

        self.counter += 1;
        if self.counter >= self.reset_interval {
            // Normalize phase to prevent overflow
            self.phase /= self.phase.norm();
            self.counter = 0;
        }

        Some(result)
    }
}
