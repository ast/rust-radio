use std::ops::{Add, Mul};

/// Decimator trait
pub trait Decimator<T> {
    fn decimate(&mut self, input: T) -> Option<T>;
}

/// A high-performance FIR decimator using a linear buffer and output subsampling.
pub struct FirDecimator<T> {
    h: Vec<f32>,           // real-valued FIR coefficients
    z: Vec<T>,             // delay buffer (linear, oversized)
    i: usize,              // current write index
    tail_threshold: usize, // reset trigger to avoid wrap
    decimation: usize,     // decimation factor
    decimation_counter: usize,
}

impl<T> FirDecimator<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>, decimation: usize) -> Self {
        let taps = h.len();
        let buffer_size = 65_536;
        assert!(
            buffer_size >= 2 * taps,
            "Delay buffer must be at least twice as long as the number of taps"
        );
        assert!(decimation > 0, "Decimation factor must be ≥ 1");

        FirDecimator {
            h,
            z: vec![T::default(); buffer_size],
            i: taps - 1, // to simulate zero-padding at start
            tail_threshold: buffer_size - taps,
            decimation,
            decimation_counter: 0,
        }
    }
}

impl<T> Decimator<T> for FirDecimator<T>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    fn decimate(&mut self, input: T) -> Option<T> {
        let n = self.h.len();
        self.z[self.i] = input;
        self.i += 1;
        self.decimation_counter += 1;

        // Copy tail back to head if needed
        if self.i >= self.tail_threshold {
            // Copy the last `n - 1` samples into the beginning
            self.z.copy_within(self.i - (n - 1)..self.i, 0);
            self.i = n - 1; // next write goes just after the copied region
        }

        if self.decimation_counter == self.decimation {
            self.decimation_counter = 0;

            let input_slice = &self.z[self.i - n..self.i];
            let y = self
                .h
                .iter()
                .zip(input_slice.iter())
                .fold(T::default(), |acc, (&h_j, &z_j)| acc + z_j * h_j);

            Some(y)
        } else {
            None
        }
    }
}
