pub mod complex_fs4_oscillator;
pub mod complex_oscillator;
pub mod complex_oscillator_fast;
pub mod file_source;
pub mod noise_source;

pub use complex_fs4_oscillator::ComplexFs4Oscillator;
pub use complex_oscillator::ComplexOscillator;
pub use complex_oscillator_fast::ComplexOscillatorFast;
pub use file_source::FileSource;
pub use noise_source::NoiseSource;

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
