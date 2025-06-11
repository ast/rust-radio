use crate::DelayLine;
use crate::ringbuffer::RingBuffer;
use std::ops::{Add, Mul};

/// Decimator trait
pub trait Decimator<T> {
    fn decimate(&mut self, input: T) -> Option<T>;
}

/// A way to chain decimators together
pub struct ChainDecimator<D1, D2, T> {
    d1: D1,
    d2: D2,
    _marker: std::marker::PhantomData<T>,
}

impl<D1, D2, T> ChainDecimator<D1, D2, T> {
    pub fn new(d1: D1, d2: D2) -> Self {
        Self {
            d1,
            d2,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<D1, D2, T> Decimator<T> for ChainDecimator<D1, D2, T>
where
    D1: Decimator<T>,
    D2: Decimator<T>,
{
    fn decimate(&mut self, x: T) -> Option<T> {
        self.d1.decimate(x).and_then(|y| self.d2.decimate(y))
    }
}

pub trait ChainableDecimator<T>: Decimator<T> + Sized {
    fn chain_with<D2>(self, other: D2) -> ChainDecimator<Self, D2, T>
    where
        D2: Decimator<T>,
    {
        ChainDecimator::new(self, other)
    }
}

impl<T, D> ChainableDecimator<T> for D where D: Decimator<T> {}

/// FirDecimator with power of two decimation factor
pub struct FirDecimator<T, const N: usize, const D: usize = 2> {
    h: [f32; N],      // real-valued FIR coefficients
    z: RingBuffer<T>, // delay buffer
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
        // Check if the write index if we should return a value
        if (self.z.write() & (D - 1)) != 0 {
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
    use std::f32::consts::PI;

    fn generate_complex_input(len: usize, sample_rate: f32, freq_hz: f32) -> Vec<Complex32> {
        let omega = 2.0 * PI * freq_hz / sample_rate;
        (0..len)
            .map(|i| {
                let phase = omega * i as f32;
                let (im, re) = phase.sin_cos();
                Complex32::new(re, im)
            })
            .collect()
    }

    #[test]
    fn test_fir_decimator() {
        let sample_rate = 768_000.0;
        let signal_freq = 50_000.0;
        let len = 768_000;

        let input = generate_complex_input(len, sample_rate, signal_freq);

        let coeffs = HB_35;

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
