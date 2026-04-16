use crate::DelayLine;
use crate::Decimator;
use crate::ringbuffer::RingBuffer;
use std::ops::{Add, Mul};

use super::dot_product;

/// FirDecimator with power of two decimation factor.
///
/// Convention: `h` is the impulse response in natural order (`h[0]` applies
/// to the newest sample) and reversed once in `new`; see the crate-level
/// note in [`super::naive::NaiveFirFilter`].
pub struct FirDecimator<T, const N: usize, const D: usize = 2> {
    h: [f32; N],      // real-valued FIR coefficients, stored reversed
    z: RingBuffer<T>, // delay buffer (oldest-first on read)
    sample: usize,    // sample counter for decimation
}

impl<T, const N: usize, const D: usize> FirDecimator<T, N, D>
where
    T: Copy + Default,
{
    pub fn new(mut h: [f32; N]) -> Self {
        assert!(D.is_power_of_two(), "D must be a power of two");

        h.reverse();
        let taps = h.len();
        FirDecimator {
            h,
            z: RingBuffer::new(taps),
            sample: 0,
        }
    }
}

impl<T, const N: usize, const D: usize> Decimator<T> for FirDecimator<T, N, D>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    #[inline]
    fn decimate(&mut self, x: T) -> Option<T> {
        self.z.push(x);
        self.sample = self.sample.wrapping_add(1);
        if (self.sample & (D - 1)) != 0 {
            return None;
        }

        Some(dot_product(&self.h, self.z.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::fir::kernels::HB_35;
    use num_complex::Complex32;
    use signals::ComplexOscillator;

    use crate::ChainableDecimator;

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
