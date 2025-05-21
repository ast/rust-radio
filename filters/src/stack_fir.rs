use std::iter::Iterator;

/// Delay line trait
pub trait DelayLine2<T> {
    fn push(&mut self, input: T);
    fn iter(&self) -> impl Iterator<Item = T>;
}

/// Stack allocated power of 2 delay line
pub struct StackLine<T, const N: usize> {
    z: [T; N],    // Delay buffer with a length of N
    index: usize, // Current write index
}

/// Iterator for StackLine
pub struct StackLineIterator<'a, T, const N: usize> {
    line: &'a StackLine<T, N>,
    index: usize,
}

/// Iterator implementation for StackLineIterator
impl<'a, T, const N: usize> Iterator for StackLineIterator<'a, T, N>
where
    T: Copy,
{
    type Item = T;

    /// Just loop back forever
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index & (N - 1);
        let value = self.line.z[index];
        self.index = index.wrapping_sub(1);
        Some(value)
    }
}

impl<T, const N: usize> StackLine<T, N>
where
    T: Copy + Default,
{
    pub fn new() -> Self {
        // Ensure N is a power of 2
        assert!(N.is_power_of_two(), "N must be a power of 2");

        StackLine {
            z: [T::default(); N],
            index: 0,
        }
    }
}

// Default
impl<T, const N: usize> Default for StackLine<T, N>
where
    T: Copy + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> DelayLine2<T> for StackLine<T, N>
where
    T: Copy + Default,
{
    fn push(&mut self, input: T) {
        self.z[self.index & (N - 1)] = input;
        self.index = self.index.wrapping_add(1);
    }

    fn iter(&self) -> impl Iterator<Item = T> {
        StackLineIterator {
            line: self,
            // self.index points to the next write position
            index: self.index - 1,
        }
    }
}

/// Naive FIR filter implementation with stack-allocated taps
/// M is the number of taps, N is the size of the buffer of the delay line
pub struct StackFirFilter<T, const M: usize, const N: usize> {
    h: [f32; M],        // real-valued taps (stack-allocated array)
    z: StackLine<T, N>, // signal buffer using StackLine
}

impl<T, const M: usize, const N: usize> StackFirFilter<T, M, N>
where
    T: Copy + Default + std::ops::Mul<f32, Output = T> + std::ops::Add<Output = T>,
{
    pub fn new(h: [f32; M]) -> Self {
        // Assert that M <= N
        assert!(
            M <= N,
            "Number of taps M must be less than or equal to buffer size N"
        );

        StackFirFilter {
            h,
            z: StackLine::<T, N>::new(),
        }
    }

    #[inline]
    pub fn filter(&mut self, input: T) -> T {
        self.z.push(input);

        let output = self
            .h
            .iter()
            .zip(self.z.iter())
            .fold(T::default(), |acc, (coeff, sample)| acc + sample * *coeff);

        output
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_line() {
        let mut line = StackLine::<i32, 32>::new();

        for _i in 0..128 {
            // Loop to make sure we wrap around
            line.push(1);
            line.push(2);
            line.push(3);
            line.push(4);

            let values: Vec<i32> = line.iter().take(4).collect();
            assert_eq!(values, vec![4, 3, 2, 1]);

            line.push(5);
            line.push(6);

            let values: Vec<i32> = line.iter().take(4).collect();
            assert_eq!(values, vec![6, 5, 4, 3]);
        }
    }

    #[test]
    fn test_simple_moving_average() {
        // Moving average filter: y[n] = (x[n] + x[n-1]) / 2
        let h = [0.5, 0.5];
        let mut filter = StackFirFilter::<f32, 2, 8>::new(h);

        let input = [1.0, 2.0, 3.0, 4.0];
        let expected = [
            0.5 * 1.0,             // x[0]
            0.5 * 2.0 + 0.5 * 1.0, // x[1], x[0]
            0.5 * 3.0 + 0.5 * 2.0, // x[2], x[1]
            0.5 * 4.0 + 0.5 * 3.0, // x[3], x[2]
        ];

        let output: Vec<f32> = input.iter().map(|&x| filter.filter(x)).collect();

        assert_eq!(output, expected);
    }
}
