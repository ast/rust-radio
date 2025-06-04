use num_complex::Complex32;
use std::f32::consts::PI;

pub struct ComplexOscillator {
    phase: f32,
    omega: f32,
}

impl ComplexOscillator {
    /// Create a new oscillator for a given frequency (Hz) and sample rate (Hz)
    pub fn new(frequency: f32, sample_rate: f32) -> Self {
        let omega = 2.0 * PI * frequency / sample_rate;

        Self { phase: 0.0, omega }
    }

    /// Reset the oscillator phase (optional external control)
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

impl Iterator for ComplexOscillator {
    type Item = Complex32;

    fn next(&mut self) -> Option<Self::Item> {
        let result = Complex32::from_polar(1.0, self.phase); // e^(j * phase)
        self.phase += self.omega;

        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI; // Normalize phase to prevent overflow
        }

        Some(result)
    }
}
