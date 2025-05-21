use crate::fir::DelayLine;
use doublemap::Doublemap;

pub struct RingBuffer<T> {
    z: Doublemap<T>, // delay buffer
    taps: usize,     // length of the delay
    i: usize,        // current write index
}

impl<T> RingBuffer<T> {
    pub fn new(taps: usize) -> Self {
        RingBuffer {
            z: Doublemap::new(512).unwrap(),
            taps,
            i: 0,
        }
    }

    // Get the current index
    #[inline(always)]
    fn mask(&self, i: usize) -> usize {
        i & (self.z.len() - 1)
    }

    // read index (taps points behind the current index)
    #[inline(always)]
    fn read_index(&self) -> usize {
        self.mask(self.i.wrapping_add(1).wrapping_sub(self.taps))
    }
}

impl<T> DelayLine<T> for RingBuffer<T> {
    fn push(&mut self, input: T) {
        self.i = self.i.wrapping_add(1);
        let masked = self.mask(self.i);
        self.z.as_mut_slice()[masked] = input;
    }

    fn as_slice(&self) -> &[T] {
        let start = self.read_index();
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
