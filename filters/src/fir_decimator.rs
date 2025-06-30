use crate::DelayLine;
use crate::decimator::Decimator;
use crate::ringbuffer::RingBuffer;
use std::ops::{Add, Mul};

/// FirDecimator with power of two decimation factor
pub struct FirDecimator<T, const N: usize, const D: usize = 2> {
    h: [f32; N],      // real-valued FIR coefficients
    z: RingBuffer<T>, // delay buffer
    sample: usize,    // sample counter for decimation
}

impl<T, const N: usize, const D: usize> FirDecimator<T, N, D>
where
    T: Copy + Default,
{
    pub fn new(h: [f32; N]) -> Self {
        // Assert D is a power of two
        assert!(D.is_power_of_two(), "D must be a power of two");

        let taps = h.len();
        FirDecimator {
            h,
            z: RingBuffer::new(taps),
            sample: 0,
        }
    }
}

#[inline]
pub fn dot_product<T>(a: &[f32], b: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    a.iter()
        .zip(b.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
}

// Impl Decimator trait for FirDecimator
impl<T, const N: usize, const D: usize> Decimator<T> for FirDecimator<T, N, D>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    #[inline]
    fn decimate(&mut self, x: T) -> Option<T> {
        // Push the new sample into the delay line

        self.z.push(x);
        self.sample = self.sample.wrapping_add(1);
        // Check if the write index if we should return a value
        if (self.sample & (D - 1)) != 0 {
            return None;
        }

        Some(dot_product(&self.h, self.z.as_slice()))
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    use crate::kernels::HB_35;
    use num_complex::Complex32;
    use signals::ComplexOscillator;

    use crate::chain_decimator::ChainableDecimator;

    #[test]
    fn test_fir_decimator() {
        let sample_rate = 768_000.0;
        let signal_freq = 50_000.0;
        let len = 768_000;

        let osc = ComplexOscillator::new(signal_freq, sample_rate);
        let input: Vec<Complex32> = osc.take(len).collect();

        let fir0: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
        let fir1: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
        let fir2: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);

        let mut chain = fir0.chain_with(fir1).chain_with(fir2);

        let output: Vec<_> = input.iter().filter_map(|&x| chain.decimate(x)).collect();

        // Check that the output is not empty
        assert!(!output.is_empty());
        assert_eq!(output.len(), input.len() / (4 * 4 * 2));
    }
}
