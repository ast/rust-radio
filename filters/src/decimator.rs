use crate::DelayLine;
use crate::ringbuffer::RingBuffer;
use std::ops::{Add, Mul};

/// Decimator trait
pub trait Decimator<T> {
    fn decimate(&mut self, input: T) -> Option<T>;
}

pub struct FirDecimator<T, const N: usize, const D: usize = 2> {
    h: [f32; N],      // real-valued FIR coefficients
    z: RingBuffer<T>, // delay buffer
}

impl<T, const N: usize, const D: usize> FirDecimator<T, N, D>
where
    T: Copy + Default,
{
    pub fn new(h: [f32; N]) -> Self {
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

        if (self.z.index() & (D - 1)) != 0 {
            return None;
        }

        Some(dot_product(&self.h, self.z.as_slice()))
    }
}
