use crate::Filter;
use std::collections::VecDeque;
use std::ops::{Add, Mul};

/// Naive FIR filter implementation, useful for testing and educational purposes.
pub struct FirFilter<T> {
    h: Vec<f32>,
    z: VecDeque<T>,
}

impl<T> FirFilter<T>
where
    T: Copy + Default,
{
    pub fn new(h: Vec<f32>) -> Self {
        let len = h.len();

        FirFilter {
            h,
            z: VecDeque::from(vec![T::default(); len]),
        }
    }
}

impl<T> Filter<T> for FirFilter<T>
where
    T: Copy + Default + Mul<f32, Output = T> + Add<Output = T>,
{
    fn filter(&mut self, x: T) -> T {
        // Push the new sample into the delay line
        self.z.push_back(x);
        self.z.pop_front();

        let (z0, z1) = self.z.as_slices();

        // Delay line iterator
        let z = z0.iter().chain(z1.iter());

        // Convolve
        let y = self
            .h
            .iter()
            .zip(z)
            .fold(T::default(), |acc, (&coeff, &sample)| acc + sample * coeff);

        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
