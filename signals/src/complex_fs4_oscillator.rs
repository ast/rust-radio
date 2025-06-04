use num_complex::Complex32;

pub struct ComplexFs4Oscillator {
    phase: usize,
    values: [Complex32; 4],
}

impl ComplexFs4Oscillator {
    /// Create a new oscillator for a given frequency (Hz) and sample rate (Hz)
    pub fn new(sign: f32) -> Self {
        assert!(
            sign == 1.0 || sign == -1.0,
            "Sign must be either 1.0 or -1.0"
        );

        let values = [
            Complex32::new(1.0, 0.0),   // 0 Hz
            Complex32::new(0.0, sign),  // 1/4 Fs
            Complex32::new(-1.0, 0.0),  // 1/2 Fs
            Complex32::new(0.0, -sign), // 3/4 Fs
        ];

        Self { phase: 0, values }
    }

    /// Reset the oscillator phase (optional external control)
    pub fn reset(&mut self) {
        self.phase = 0;
    }
}

impl Iterator for ComplexFs4Oscillator {
    type Item = Complex32;

    fn next(&mut self) -> Option<Self::Item> {
        // Rotate and return the complex value
        let result = self.values[self.phase & 0b11];
        self.phase += 1;
        Some(result)
    }
}
