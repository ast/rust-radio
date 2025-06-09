use super::Doublemap;

#[derive(Debug)]
pub struct RingBuffer<T> {
    buffer: Doublemap<T>, // delay buffer
    write: usize,         // write position
    read: usize,          // read position
}

impl<T: Copy> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        RingBuffer {
            // Doublemap will only allow power of two sizes
            buffer: Doublemap::new(capacity).expect("Failed to create Doublemap"),
            write: 0,
            read: 0,
        }
    }

    fn mask(&self, index: usize) -> usize {
        index & (self.buffer.len() - 1)
    }

    pub fn emtpy(&self) -> bool {
        self.read == self.write
    }

    pub fn size(&self) -> usize {
        return self.write.wrapping_sub(self.read);
    }

    pub fn full(&self) -> bool {
        self.size() == self.buffer.len()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn push(&mut self, item: T) {
        assert!(!self.full());
        let index = self.mask(self.write);
        self.buffer.as_mut_slice()[index] = item;
        self.write = self.write.wrapping_add(1);
    }

    pub fn shift(&mut self) -> Option<T> {
        if self.emtpy() {
            return None;
        }
        let index = self.mask(self.read);
        let item = self.buffer.as_slice()[index];
        self.read = self.read.wrapping_add(1);
        Some(item)
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let capacity = 1024;
        let mut ring_buffer = RingBuffer::<u8>::new(capacity);

        assert_eq!(ring_buffer.size(), 0);
        // It will be increased to page size
        assert_eq!(ring_buffer.len(), 4096);

        ring_buffer.push(1);

        assert_eq!(ring_buffer.size(), 1);
        assert!(!ring_buffer.emtpy());

        // Push ten more
        for i in 0..10 {
            ring_buffer.push(i);
        }

        assert_eq!(ring_buffer.size(), 11);

        let item = ring_buffer.shift();
        assert_eq!(item, Some(1));

        let item = ring_buffer.shift();
        assert_eq!(item, Some(0));

        let item = ring_buffer.shift();
        assert_eq!(item, Some(1));

        // Add more tests for push, pop, and other functionalities
    }
}
