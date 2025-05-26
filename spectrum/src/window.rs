/// Hamming window
use std::f32::consts::PI;

pub fn hamming(n: usize, N: usize) -> f32 {
    0.54 - 0.46 * (2.0 * PI * n as f32 / (N - 1) as f32).cos()
}

pub fn hamming_window(N: usize) -> Vec<f32> {
    (0..N).map(|n| hamming(n, N)).collect()
}

pub fn hann(n: usize, N: usize) -> f32 {
    0.5 * (1.0 - (2.0 * PI * n as f32 / (N - 1) as f32).cos())
}

/// Hanning window
pub fn hann_window(N: usize) -> Vec<f32> {
    (0..N).map(|n| hann(n, N)).collect()
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_LEN: usize = 32768;

    #[test]
    fn test_hamming_window() {
        let size = WINDOW_LEN;
        let window = hamming_window(size);
        assert_eq!(window.len(), size);
    }

    #[test]
    fn test_hann_window() {
        let size = WINDOW_LEN;
        let window = hann_window(size);
        assert_eq!(window.len(), size);
    }
}
