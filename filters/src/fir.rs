use std::ops::{Add, Mul};

/// Simple filter trait
pub trait Filter<T> {
    fn filter(&mut self, input: T) -> T;
}

/// Delay line trait
pub trait DelayLine<T> {
    fn push(&mut self, input: T);
    fn as_slice(&self) -> &[T];
}

pub struct BufferLine<T> {
    z: Vec<T>,   // delay buffer
    taps: usize, // length of the delay
    i: usize,    // current write index
}

impl<T> BufferLine<T>
where
    T: Copy + Default,
{
    pub fn new(taps: usize) -> Self {
        const BUFFER_SIZE: usize = 65_536;

        BufferLine {
            z: vec![T::default(); BUFFER_SIZE],
            taps,
            i: taps - 1,
        }
    }
}

impl<T> DelayLine<T> for BufferLine<T>
where
    T: Copy + Default,
{
    fn push(&mut self, input: T) {
        self.i += 1;
        self.z[self.i] = input;

        if self.i >= self.z.len() - 1 {
            // Copy last `taps` samples into the start
            let src_start = self.i - (self.taps - 1);
            let src_end = self.i + 1;
            self.z.copy_within(src_start..src_end, 0);
            self.i = self.taps - 1;
        }
    }

    fn as_slice(&self) -> &[T] {
        &self.z[self.i + 1 - self.taps..=self.i]
    }
}

/// Naive FIR filter implementation
pub struct FirFilter<T> {
    h: Vec<f32>, // real-valued taps
    z: Vec<T>,   // signal buffer
    i: usize,    // circular write index
}

impl<T> FirFilter<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>) -> Self {
        let len = h.len();
        FirFilter {
            h,
            z: vec![T::default(); len],
            i: 0,
        }
    }
}

impl<T> Filter<T> for FirFilter<T>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    fn filter(&mut self, x: T) -> T {
        let n = self.z.len();
        self.z[self.i] = x;

        let mut y = T::default();
        for (j, &h_j) in self.h.iter().enumerate() {
            let zi = (self.i + n - j) % n;
            y = y + self.z[zi] * h_j;
        }

        self.i = (self.i + 1) % n;
        y
    }
}

/// A higher-performance FIR filter using a large linear buffer
/// to avoid modulo operations and improve cache locality.
pub struct FirFilter2<T> {
    h: Vec<f32>,           // real-valued FIR coefficients
    z: Vec<T>,             // large delay buffer
    i: usize,              // current write index
    tail_threshold: usize, // point at which we reset the buffer
}

impl<T> FirFilter2<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>) -> Self {
        let taps = h.len();
        let buffer_size = 65_536; // ~256KB for Complex32
        assert!(
            buffer_size >= 2 * taps,
            "Delay buffer must be at least twice as long as the number of taps"
        );

        FirFilter2 {
            h,
            z: vec![T::default(); buffer_size],
            i: taps - 1,
            tail_threshold: buffer_size - taps,
        }
    }
}

impl<T> crate::Filter<T> for FirFilter2<T>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    fn filter(&mut self, x: T) -> T {
        let n = self.h.len();

        // Write the new sample
        self.z[self.i] = x;
        self.i += 1;

        // Compute convolution over contiguous slice
        let input_slice = &self.z[self.i - n..self.i];
        let y = self
            .h
            .iter()
            .zip(input_slice.iter())
            .fold(T::default(), |acc, (&h_j, &z_j)| acc + z_j * h_j);

        // If we're near the end, shift last taps into the start
        if self.i >= self.tail_threshold {
            // Copy the last `n - 1` samples into the beginning
            self.z.copy_within(self.i - (n - 1)..self.i, 0);
            self.i = n - 1; // next write goes just after the copied region
        }

        y
    }
}

/// FirFilter3
pub struct FirFilter3<T> {
    h: Vec<f32>,      // real-valued FIR coefficients
    z: BufferLine<T>, // delay buffer
}

impl<T> FirFilter3<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>) -> Self {
        let taps = h.len();
        FirFilter3 {
            h,
            z: BufferLine::new(taps),
        }
    }
}

pub fn dot_product<T>(a: &[f32], b: &[T]) -> T
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    a.iter()
        .zip(b.iter())
        .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff)
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

    // fn generate_coefficients(n: usize) -> Vec<f32> {
    //     let norm = 1.0 / n as f32;
    //     vec![norm; n] // simple moving average filter
    // }

    fn approx_eq_f32(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() < tol
    }

    fn approx_eq_complexf32(a: Complex32, b: Complex32, tol: f32) -> bool {
        (a - b).norm() < tol
    }

    fn apply_filter(filter: &mut impl Filter<f32>, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| filter.filter(x)).collect()
    }

    #[test]
    fn test_impulse_identity() {
        // Impulse response: [1.0] — should just copy the input
        let h = vec![1.0];
        let mut filter = FirFilter::new(h);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let expected = input.clone();

        let output = apply_filter(&mut filter, &input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_simple_moving_average() {
        // Moving average filter: y[n] = (x[n] + x[n-1]) / 2
        let h = vec![0.5, 0.5];
        let mut filter = FirFilter::new(h);

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
    fn test_zero_input() {
        let h = vec![0.3, 0.6, 0.1];
        let mut filter = FirFilter::new(h);
        let input = vec![0.0; 5];
        let expected = vec![0.0; 5];

        let output = apply_filter(&mut filter, &input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_firfilter2_matches_naive_fir() {
        let taps = vec![0.25, 0.5, 0.25];
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let mut naive = FirFilter::new(taps.clone());
        let mut optimized = FirFilter2::new(taps);

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

        let input = generate_complex_input(len, sample_rate, signal_freq);

        let mut naive = FirFilter::new(taps.clone());
        let mut optimized = FirFilter2::new(taps.clone());
        let mut optimized2 = FirFilter3::new(taps);

        let out_naive: Vec<_> = input.iter().map(|&x| naive.filter(x)).collect();
        let out_optimized: Vec<_> = input.iter().map(|&x| optimized.filter(x)).collect();
        let out_optimized2: Vec<_> = input.iter().map(|&x| optimized2.filter(x)).collect();

        assert_eq!(out_naive.len(), out_optimized.len());
        assert_eq!(out_naive.len(), out_optimized2.len());

        for (i, (&a, &b)) in out_naive.iter().zip(out_optimized.iter()).enumerate() {
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

    #[test]
    fn test_buffer_line_push_and_slice() {
        let delay = 4;
        let mut line = BufferLine::<f32>::new(delay);

        // Expect the delay line to be empty initially
        let slice = line.as_slice();
        let expected = vec![0.0; delay];
        assert_eq!(slice, expected);

        line.push(1.0);
        line.push(2.0);
        line.push(3.0);

        // Test
        // The delay line should now be [0.0, 1.0, 2.0, 3.0]
        let slice = line.as_slice();
        let expected = [0.0, 1.0, 2.0, 3.0];
        assert_eq!(slice, expected);

        line.push(4.0);
        // Now delay line should be [1.0, 2.0, 3.0, 4.0]
        let slice = line.as_slice();
        let expected = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(slice, expected);
    }
}
