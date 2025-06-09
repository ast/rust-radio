use crate::DelayLine;

use doublemap::Doublemap;

#[derive(Debug)]
pub struct RingBuffer<T> {
    z: Doublemap<T>, // delay buffer
    taps: usize,     // length of the delay
    i: usize,        // current write index
    imask: usize,    // mask for the index
}

impl<T: Copy> RingBuffer<T> {
    pub fn new(taps: usize) -> Self {
        let doublemap =
            Doublemap::<T>::new(taps.next_power_of_two()).expect("Failed to create Doublemap");
        let len = doublemap.len();

        RingBuffer {
            z: doublemap,
            taps,
            i: 0,
            imask: len - 1,
        }
    }

    #[inline]
    pub fn index(&self) -> usize {
        self.i
    }

    // read index (taps points behind the current index)
    #[inline]
    pub fn start(&self) -> usize {
        self.mask(self.i.wrapping_sub(self.taps))
    }

    // Get the current index
    #[inline]
    fn mask(&self, i: usize) -> usize {
        i & self.imask
    }
}

impl<T: Copy> DelayLine<T> for RingBuffer<T> {
    #[inline]
    fn push(&mut self, input: T) {
        let masked = self.mask(self.i);
        self.z.as_mut_slice()[masked] = input;
        self.i = self.i.wrapping_add(1);
    }
    #[inline]
    fn as_slice(&self) -> &[T] {
        let start = self.start();
        // Double buffering assures that the read index is always valid
        &self.z.as_slice()[start..start + self.taps]
    }
}

// Test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ringbuffer() {
        let taps = 7;
        let mut ringbuffer = RingBuffer::<u8>::new(taps);

        // should have 7 taps zeros
        assert_eq!(ringbuffer.as_slice(), &[0, 0, 0, 0, 0, 0, 0]);

        for j in 0..(32768 * 8) {
            for i in 0..taps {
                ringbuffer.push(i as u8);
            }
            assert_eq!(ringbuffer.as_slice(), &[0, 1, 2, 3, 4, 5, 6]);
        }
    }
}
