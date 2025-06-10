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

    // Mask the index to fit into the buffer size
    fn mask(&self, index: usize) -> usize {
        index & (self.buffer.len() - 1)
    }

    // is the buffer empty?
    pub fn is_empty(&self) -> bool {
        self.read == self.write
    }

    // is the buffer full?
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    // how many items are available to read in the buffer?
    pub fn len(&self) -> usize {
        self.write.wrapping_sub(self.read)
    }

    // how many items are available to write in the buffer?
    pub fn available(&self) -> usize {
        self.capacity() - self.len()
    }

    pub fn capacity(&self) -> usize {
        // Rename len in doublebuffer
        self.buffer.len()
    }

    // push an item into the buffer
    pub fn push(&mut self, item: T) {
        assert!(!self.is_full());
        let index = self.mask(self.write);
        self.buffer.as_mut_slice()[index] = item;
        self.write = self.write.wrapping_add(1);
    }

    // pop an item from the buffer
    pub fn shift(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let index = self.mask(self.read);
        let item = self.buffer.as_slice()[index];
        self.read = self.read.wrapping_add(1);
        Some(item)
    }

    pub fn as_slice(&self) -> &[T] {
        let start = self.mask(self.read);
        let end = start + self.len();
        let slice = self.buffer.as_slice();

        // The double mapped memory buffer assures that the slice is
        // valid.
        &slice[start..end]
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let start = self.mask(self.write);
        let end = start + self.available();
        let slice = self.buffer.as_mut_slice();

        // The double mapped memory buffer assures that the slice is
        // valid.
        &mut slice[start..end]
    }

    pub fn produce(&mut self, num: usize) {
        assert!(num <= self.available());
        self.write = self.write.wrapping_add(num);
    }

    pub fn consume(&mut self, num: usize) {
        assert!(num <= self.len(), "Cannot consume more than available");
        self.read = self.read.wrapping_add(num);
    }
}

impl<T: Copy> Iterator for RingBuffer<T> {
    type Item = T;

    // Next
    fn next(&mut self) -> Option<Self::Item> {
        self.shift()
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

        assert_eq!(ring_buffer.len(), 0);
        // It will be increased to page size
        assert_eq!(ring_buffer.capacity(), 4096);

        ring_buffer.push(1);

        assert_eq!(ring_buffer.len(), 1);
        assert!(!ring_buffer.is_empty());

        // Push ten more
        for i in 0..10 {
            ring_buffer.push(i);
        }

        assert_eq!(ring_buffer.len(), 11);

        let item = ring_buffer.shift();
        assert_eq!(item, Some(1));

        let item = ring_buffer.shift();
        assert_eq!(item, Some(0));

        let item = ring_buffer.shift();
        assert_eq!(item, Some(1));

        let slice = ring_buffer.as_slice();
        assert_eq!(slice.len(), 8);

        // Check the contents of the slice
        for (i, &value) in slice.iter().enumerate() {
            assert_eq!(value, (i + 2) as u8);
        }

        // Add more tests for push, pop, and other functionalities
    }

    #[test]
    fn test_ring_buffer_as_slice() {
        let capacity = 1024;
        let mut ring_buffer = RingBuffer::<u8>::new(capacity);

        assert_eq!(ring_buffer.len(), 0);
        // It will be increased to page size
        assert_eq!(ring_buffer.capacity(), 4096);

        {
            // borrow as mut slice
            let slice = ring_buffer.as_mut_slice();
            assert_eq!(slice.len(), 4096);

            // Fill the first half with values
            for i in 0..slice.len() / 2 {
                slice[i] = (i % 256) as u8;
            }
        }

        // Tell that we have produced half of the buffer
        ring_buffer.produce(4096 / 2);

        // Check that the first half is filled with values
        let slice = ring_buffer.as_slice();
        assert_eq!(slice.len(), 2048);
    }
}
