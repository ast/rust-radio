use num_complex::Complex32;

pub struct NoiseSource {
    std_dev: f32,
}

impl NoiseSource {
    /// Create a new oscillator for a given frequency (Hz) and sample rate (Hz)
    pub fn new(dbfs: f32) -> Self {
        let rms = 10f32.powf(dbfs / 20.0);
        let std_dev = rms / 2.0f32.sqrt();

        Self { std_dev }
    }

    /// Reset the oscillator power
    pub fn reset(&mut self, std_dev: f32) {
        self.std_dev = std_dev;
    }
}

impl Iterator for NoiseSource {
    type Item = Complex32;

    fn next(&mut self) -> Option<Self::Item> {
        // Box-Muller transform
        let u1 = fastrand::f32(); // uniform in (0, 1)
        let u2 = fastrand::f32();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        let (z1, z2) = (r * theta.cos(), r * theta.sin());

        Some(Complex32::new(z1 * self.std_dev, z2 * self.std_dev))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_source() {
        let dbfs = -60.0; // -60 dBFS
        let num_samples = 10000;

        let noise_source = NoiseSource::new(dbfs);
        let output: Vec<Complex32> = noise_source.take(num_samples).collect();

        assert_eq!(output.len(), num_samples, "Output length mismatch");

        // Check the mean and standard deviation of the generated noise
        let mean = output.iter().map(|c| c.norm()).sum::<f32>() / num_samples as f32;
        let std_dev = (output
            .iter()
            .map(|c| c.norm())
            .map(|x| (x - mean).powi(2))
            .sum::<f32>()
            / num_samples as f32)
            .sqrt();

        // The mean should be close to 0 and std_dev should be close to the expected value
        assert!((mean - 0.0).abs() < 0.1, "Mean is not close to 0");
        assert!(
            (std_dev - (10f32.powf(dbfs / 20.0) / 2.0_f32.sqrt())).abs() < 0.1,
            "Standard deviation is not as expected"
        );
    }
}
