use std::ops::{Add, Mul};

use crate::DelayLine;
use crate::Filter;
use crate::ringbuffer::RingBuffer;

/// Dot product of FIR coefficients and delay line samples.
#[inline]
pub fn dot_product<T>(h: &[f32], z: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    h.iter()
        .zip(z.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
}

/// FirFilter3
pub struct FirFilter3<T> {
    h: Vec<f32>,      // real-valued FIR coefficients
    z: RingBuffer<T>, // delay buffer
}

impl<T> FirFilter3<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>) -> Self {
        let taps = h.len();
        FirFilter3 {
            h,
            z: RingBuffer::new(taps),
        }
    }
}

// Impl Filter trait for FirFilter3
impl<T> Filter<T> for FirFilter3<T>
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
    use num_complex::Complex32;
    use signals::ComplexOscillator;

    fn approx_eq_f32(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn approx_eq_complexf32(a: Complex32, b: Complex32, tol: f32) -> bool {
        (a - b).norm() < tol
    }

    #[test]
    fn test_firfilter3_matches_naive_fir() {
        let taps = vec![0.25, 0.5, 0.25];
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let mut naive = FirFilter::new(taps.clone());
        let mut optimized = FirFilter3::new(taps);

        let out_naive: Vec<f32> = input.iter().map(|&x| naive.filter(x)).collect();
        let out_optimized: Vec<f32> = input.iter().map(|&x| optimized.filter(x)).collect();

        assert_eq!(out_naive.len(), out_optimized.len());

        for (i, (&a, &b)) in out_naive.iter().zip(out_optimized.iter()).enumerate() {
            assert!(
                approx_eq_f32(a, b, 1e-6),
                "Mismatch at index {i}: naive = {a}, optimized = {b}"
            );
        }
    }

    #[test]
    fn test_firfilter2_matches_naive_fir_long_signal() {
        let sample_rate = 768_000.0;
        let signal_freq = 50_000.0;
        let taps = vec![0.1, 0.25, 0.5, 0.25, 0.1];
        let len = 768_000;

        let osc = ComplexOscillator::new(signal_freq, sample_rate);
        let input = osc.take(len).collect::<Vec<_>>();

        let mut naive = FirFilter::new(taps.clone());
        let mut optimized2 = FirFilter3::new(taps);

        let out_naive: Vec<_> = input.iter().map(|&x| naive.filter(x)).collect();
        let out_optimized2: Vec<_> = input.iter().map(|&x| optimized2.filter(x)).collect();

        assert_eq!(out_naive.len(), out_optimized2.len());

        for (i, (&a, &b)) in out_naive.iter().zip(out_optimized2.iter()).enumerate() {
            assert!(
                approx_eq_complexf32(a, b, 1e-6),
                "Mismatch at sample {i}: naive = {a}, optimized = {b}"
            );
        }
    }
}
