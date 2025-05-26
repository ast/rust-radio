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

#[cfg(test)]
mod tests {
    use super::*;

    use num_complex::Complex32;

    #[test]
    fn test_oscillator() {
        let frequency = 50_000f32;
        let sample_rate = 768_000f32;

        let osc_slow = ComplexOscillator::new(frequency, sample_rate);
        let osc_fast = ComplexOscillatorFast::new(frequency, sample_rate, 512);

        let output0 = osc_slow.take(10000).collect::<Vec<Complex32>>();
        let output1 = osc_fast.take(10000).collect::<Vec<Complex32>>();

        assert_eq!(output0.len(), output1.len());

        for (i, (a, b)) in output0.iter().zip(output1.iter()).enumerate() {
            assert!(
                (a - b).norm() < 1e-3,
                "Mismatch at index {}: {:?} != {:?}",
                i,
                a,
                b
            );
        }
    }

    #[test]
    fn test_oscillator_fs4() {
        let sample_rate = 768_000f32;
        let frequency = sample_rate / 4f32;

        let num_samples = 10000;

        let osc_slow = ComplexOscillator::new(frequency, sample_rate);
        let osc_fast = ComplexOscillatorFast::new(frequency, sample_rate, 512);
        let osc_fs4 = ComplexFs4Oscillator::new(1.0);

        let output0 = osc_slow.take(num_samples).collect::<Vec<Complex32>>();
        let output1 = osc_fast.take(num_samples).collect::<Vec<Complex32>>();
        let output2 = osc_fs4.take(num_samples).collect::<Vec<Complex32>>();

        assert_eq!(output0.len(), output1.len(), "Output lengths do not match");
        assert_eq!(output0.len(), output2.len(), "Output lengths do not match");

        for (i, (a, b)) in output0.iter().zip(output1.iter()).enumerate() {
            assert!(
                (a - b).norm() < 1e-3,
                "Mismatch at index {}: {:?} != {:?}",
                i,
                a,
                b
            );
        }

        for (i, (a, b)) in output0.iter().zip(output2.iter()).enumerate() {
            assert!(
                (a - b).norm() < 1e-3,
                "Mismatch at index {}: {:?} != {:?}",
                i,
                a,
                b
            );
        }

        // Fs2 oscillator
        let signs: Vec<f32> = [1f32, -1f32]
            .into_iter()
            .cycle()
            .take(num_samples)
            .collect();

        assert!(
            signs.len() == output2.len(),
            "Signs length does not match output length"
        );

        assert_eq!(signs[0], 1f32);
        assert_eq!(signs[1], -1f32);
        assert_eq!(signs[2], 1f32);
        assert_eq!(signs[3], -1f32);
    }
}
