use std::ops::{Add, Mul};

use crate::DelayLine;
use crate::Filter;
use crate::ringbuffer::RingBuffer;

/// FirFilter3
pub struct FirFilter4<T, const N: usize> {
    h: [f32; N],      // real-valued FIR coefficients
    z: RingBuffer<T>, // delay buffer
}

impl<T, const N: usize> FirFilter4<T, N>
where
    T: Copy + Default,
{
    pub fn new(h: [f32; N]) -> Self {
        let taps = h.len();
        FirFilter4 {
            h,
            z: RingBuffer::new(taps),
        }
    }
}

#[inline]
pub fn dot_product<T, const N: usize>(h: &[f32; N], z: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    h.iter()
        .zip(z.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
}

// Impl Filter trait for FirFilter4
impl<T, const N: usize> Filter<T> for FirFilter4<T, N>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    fn filter(&mut self, x: T) -> T {
        // Push the new sample into the delay line
        self.z.push(x);
        dot_product(&self.h, self.z.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::FirFilter;
    use crate::kernels::HB_35;

    use num_complex::Complex32;
    use signals::ComplexOscillator;

    fn approx_eq_complexf32(a: Complex32, b: Complex32, tol: f32) -> bool {
        (a - b).norm() < tol
    }

    fn apply_filter(filter: &mut impl Filter<f32>, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| filter.filter(x)).collect()
    }

    #[test]
    fn test_impulse_identity() {
        // Impulse response: [1.0] — should just copy the input
        let h = [1.0];

        let mut filter = FirFilter4::new(h);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let expected = input.clone();

        let output = apply_filter(&mut filter, &input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_simple_moving_average() {
        // Moving average filter: y[n] = (x[n] + x[n-1]) / 2
        let h = [0.5, 0.5];
        let mut filter = FirFilter4::new(h);

        let input = [1.0, 2.0, 3.0, 4.0];
        let expected = [
            0.5 * 1.0,             // x[0]
            0.5 * 2.0 + 0.5 * 1.0, // x[1], x[0]
            0.5 * 3.0 + 0.5 * 2.0, // x[2], x[1]
            0.5 * 4.0 + 0.5 * 3.0, // x[3], x[2]
        ];

        let output = apply_filter(&mut filter, &input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_firfilter4_matches_naive_fir_long_signal() {
        let sample_rate = 768_000.0;
        let signal_freq = 50_000.0;

        let taps = HB_35;
        let len = 768_000;

        let osc = ComplexOscillator::new(signal_freq, sample_rate);
        let input = osc.take(len).collect::<Vec<_>>();

        let mut naive = FirFilter::new(taps.to_vec());
        let mut optimized2 = FirFilter4::new(taps);

        let out_naive: Vec<_> = input.iter().map(|&x| naive.filter(x)).collect();
        let out_optimized2: Vec<_> = input.iter().map(|&x| optimized2.filter(x)).collect();

        assert_eq!(out_naive.len(), out_optimized2.len());
        assert_eq!(out_naive.len(), out_optimized2.len());

        for (i, (&a, &b)) in out_naive.iter().zip(out_optimized2.iter()).enumerate() {
            assert!(
                approx_eq_complexf32(a, b, 1e-6),
                "Mismatch at sample {i}: naive = {a}, optimized = {b}"
            );
        }

        for (i, (&a, &b)) in out_naive.iter().zip(out_optimized2.iter()).enumerate() {
            assert!(
                approx_eq_complexf32(a, b, 1e-6),
                "Mismatch at sample {i}: naive = {a}, optimized2 = {b}"
            );
        }
    }
}
