use rustfft::algorithm::Radix4;
use rustfft::{Fft, FftDirection, num_complex::Complex32};

use crate::window;

pub struct Analyzer {
    fft: Radix4<f32>,
    window: Vec<f32>,
    buffer: Vec<Complex32>,
    sample_rate: f32,
    correction: f32,
}

impl Analyzer {
    pub fn new(size: usize, sample_rate: f32) -> Self {
        assert!(size.is_power_of_two(), "Size must be a power of two");

        let fft = Radix4::<f32>::new(size, FftDirection::Forward);

        let mut window = window::hann_window(size);

        // Window power
        let mut window_power: f32 = window.iter().map(|&x| x * x).sum();
        window_power /= window.len() as f32;

        window.iter_mut().enumerate().for_each(|(n, x)| {
            // Multiply each element by (-1)^n. Effectively shifting by
            // sample rate/2 and putting the DC component in bin N.
            *x *= (-1.0f32).powi(n as i32);
        });

        // Add more corrections later
        // Correction for FFT length
        let fft_n_correction = -20.0 * (size as f32).log10();
        // 10 because it's already sum of squares
        let window_correction = -10.0 * window_power.log10();

        Analyzer {
            fft,
            window,
            buffer: vec![Complex32::default(); size],
            sample_rate,
            correction: fft_n_correction + window_correction,
        }
    }

    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn dc_bin(&self) -> usize {
        // DC component is at bin N
        self.size() / 2
    }

    /// Calculate the bin index for a given frequency
    pub fn bin(&self, frequency: f32) -> usize {
        assert!(
            frequency >= 0.0 && frequency <= self.sample_rate / 2.0,
            "Frequency must be in the range [0, sample_rate/2]"
        );

        let bin =
            (frequency / self.sample_rate * self.size() as f32).round() as usize + self.dc_bin();

        assert!(bin < self.size(), "Bin index out of bounds");

        bin
    }

    pub fn frequency(&self, bin: usize) -> f32 {
        assert!(bin < self.size(), "Bin index out of bounds");

        (bin as f32 - self.dc_bin() as f32) * self.sample_rate / self.size() as f32
    }

    pub fn process(&mut self, input: &[Complex32], output: &mut [f32]) {
        assert_eq!(
            input.len(),
            self.buffer.len(),
            "Input size must match buffer size"
        );

        // Apply the window function (this will also shift the DC component to bin N)
        input.iter().enumerate().for_each(|(i, &sample)| {
            self.buffer[i] = sample * Complex32::new(self.window[i], 0.0);
        });

        // Perform FFT
        self.fft.process(&mut self.buffer);

        for (i, &sample) in self.buffer.iter().enumerate() {
            // Calculate magnitude squared using complex conjugate and store in output
            let mut mag_squared = (sample * sample.conj()).re; // This gives us the magnitude squared (real part)

            // Check if magnitude squared is non-negative
            //assert!(mag_squared >= 0.0, "Magnitude squared must be non-negative");
            if (mag_squared < 0.0) {
                eprintln!(
                    "Warning: Negative magnitude squared at index {}: {}",
                    i, mag_squared
                );
                // Set to zero or handle as needed
                mag_squared = mag_squared.abs(); // Use absolute value to avoid negative values
            }

            output[i] = if mag_squared == 0.0 {
                f32::NEG_INFINITY // Use NEG_INFINITY for zero magnitude (or you can use 0.0)
            } else {
                // 10 because mag is already squared
                let db = 10.0 * mag_squared.log10();
                db + self.correction // Apply correction factor
            }
        }
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    use rustfft::{FftPlanner, num_complex::Complex};
    use signals::ComplexOscillator;

    #[test]
    fn test_analyzer_creation() {
        // Perform a forward FFT of size 1234

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(2048);

        let mut buffer = vec![
            Complex {
                re: 0.0f32,
                im: 0.0f32
            };
            2048
        ];
        fft.process(&mut buffer);
    }

    // Test process
    #[test]
    fn test_analyzer_bin_frequency() {
        let size = 32768; // Must be a power of two
        let sample_rate = 768_000f32; // Sample rate in Hz

        let analyzer = Analyzer::new(size, sample_rate);

        // Test bin calculation

        let bin = analyzer.bin(50_000.0f32);
        let frequency = analyzer.frequency(bin);

        assert!(
            (frequency - 50_000f32).abs() < 10.0,
            "Frequency should match input frequency"
        );
    }

    // Test process
    #[test]
    fn test_analyzer_process() {
        let size = 32768; // Must be a power of two
        let sample_rate = 768_000f32; // Sample rate in Hz

        let osc = ComplexOscillator::new(50_000f32, sample_rate);

        // Input to the analyzer
        let input: Vec<Complex32> = osc.take(size).collect();
        let mut output = vec![f32::default(); size];

        let mut analyzer = Analyzer::new(size, sample_rate);

        // Process the input through the analyzer
        analyzer.process(&input, &mut output);

        assert_eq!(output.len(), size);

        // Find the max value in the output and its index
        let (max_bin, max_value) = output.iter().enumerate().fold(
            (0, f32::NEG_INFINITY),
            |(max_idx, max_val), (idx, &val)| {
                if val > max_val {
                    (idx, val)
                } else {
                    (max_idx, max_val)
                }
            },
        );

        // Check that max value is close to 0 dB
        assert!(
            max_value > -3.0 && max_value < 1.0,
            "Max value in output should be close to 0 dB, found: {}",
            max_value
        );

        // Calculate frequency for max_bin
        let frequency = analyzer.frequency(max_bin);

        // Check that frequency is close to 50 kHz
        assert!(
            (frequency - 50_000.0).abs() < 10.0,
            "Frequency at max bin should be close to 50 kHz, found: {}",
            frequency
        );
    }
}
