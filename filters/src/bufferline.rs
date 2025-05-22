use crate::DelayLine;

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
    #[inline]
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

    #[inline]
    fn as_slice(&self) -> &[T] {
        &self.z[self.i + 1 - self.taps..=self.i]
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

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
