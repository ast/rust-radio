use crate::DelayLine;

use doublemap::Doublemap;

unsafe impl<T: Send> Send for RingBuffer<T> {}

#[derive(Debug)]
pub struct RingBuffer<T> {
    buffer: Doublemap<T>, // delay buffer
    capacity: usize,      // capacity of the buffer (must be a power of two)
    taps: usize,          // length of the delay
    write: usize,         // current write index
}

impl<T: Copy> RingBuffer<T> {
    pub fn new(taps: usize) -> Self {
        let buffer =
            Doublemap::<T>::new(taps.next_power_of_two()).expect("Failed to create Doublemap");

        let capacity = buffer.capacity();

        RingBuffer {
            buffer,
            capacity,
            taps,
            write: 0,
        }
    }

    // Get the current index
    #[inline]
    fn mask(&self, i: usize) -> usize {
        i & (self.capacity - 1)
    }

    // read index (taps points behind the current index)
    #[inline]
    pub fn start(&self) -> usize {
        self.mask(self.write.wrapping_sub(self.taps))
    }
}

impl<T: Copy> DelayLine<T> for RingBuffer<T> {
    #[inline]
    fn push(&mut self, input: T) {
        let masked = self.mask(self.write);

        // No range check needed here, as we know the mask is always within bounds
        unsafe {
            let ptr = self.buffer.as_mut_ptr().add(masked);
            *ptr = input;
        }
        self.write = self.write.wrapping_add(1);

        // self.buffer.as_mut_slice()[masked] = input;
    }

    #[inline(always)]
    fn as_slice(&self) -> &[T] {
        // Find start by looking back `taps` positions from the current write index
        let start = self.mask(self.write.wrapping_sub(self.taps));
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().add(start), self.taps) }
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    use num_complex::Complex32;

    #[test]
    fn test_ringbuffer() {
        let taps = 7;
        let mut ringbuffer = RingBuffer::<u8>::new(taps);

        // should have 7 taps zeros
        assert_eq!(ringbuffer.as_slice(), &[0, 0, 0, 0, 0, 0, 0]);

        for _j in 0..(32768 * 8) {
            for i in 0..taps {
                ringbuffer.push(i as u8);
            }
            assert_eq!(ringbuffer.as_slice(), &[0, 1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn test_ringbuffer_c32() {
        let taps = 32;
        let mut ringbuffer = RingBuffer::<Complex32>::new(taps);

        // assert all zero
        assert_eq!(ringbuffer.as_slice(), &[Complex32::new(0.0, 0.0); 32]);

        for _j in 0..(32768 * 8) {
            for i in 0..taps {
                ringbuffer.push(Complex32::new(i as f32, 0.0));
            }
        }

        // Check the last 32 values
        let expected: Vec<Complex32> = (0..taps).map(|i| Complex32::new(i as f32, 0.0)).collect();
        assert_eq!(ringbuffer.as_slice(), expected.as_slice());
    }
}
