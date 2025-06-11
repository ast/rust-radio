use super::Doublemap;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;

// Inner ring buffer
#[derive(Debug)]
struct RingBuffer<T> {
    buffer: Doublemap<T>, // delay buffer
    capacity: usize,      // capacity of the buffer (must be a power of two)
    write: usize,         // write position
    read: usize,          // read position
}

impl<T: Copy> RingBuffer<T> {
    fn new(capacity: usize) -> Self {
        let buffer = Doublemap::new(capacity).expect("Failed to create Doublemap");
        let capacity = buffer.capacity();

        RingBuffer {
            // Doublemap will only allow power of two sizes
            buffer,
            capacity,
            write: 0,
            read: 0,
        }
    }

    // Mask the index to fit into the buffer size
    fn mask(&self, index: usize) -> usize {
        index & (self.capacity - 1)
    }

    // Available items to read in the buffer
    fn len(&self) -> usize {
        self.write.wrapping_sub(self.read)
    }

    // Available items to write in the buffer
    fn free(&self) -> usize {
        self.capacity() - self.len()
    }

    // Total capacity of the buffer
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn is_empty(&self) -> bool {
        self.read == self.write
    }

    fn is_full(&self) -> bool {
        self.capacity == self.len()
    }

    pub fn produce(&mut self, num: usize) {
        assert!(num <= self.free());
        self.write = self.write.wrapping_add(num);
    }

    pub fn consume(&mut self, num: usize) {
        assert!(num <= self.len(), "Cannot consume more than available");
        self.read = self.read.wrapping_add(num);
    }

    pub fn as_slice(&self) -> &[T] {
        let start = self.mask(self.read);
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().add(start), self.len()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let start = self.mask(self.write);
        unsafe { std::slice::from_raw_parts_mut(self.buffer.as_mut_ptr().add(start), self.free()) }
    }
}

struct Shared<T> {
    buffer: Mutex<RingBuffer<T>>,
    condvar: Condvar,
}

// Producer part
pub struct Producer<T> {
    shared: Arc<Shared<T>>,
}

impl<T: Copy> Producer<T> {
    pub fn is_full(&self) -> bool {
        let buffer = self.shared.buffer.lock();
        buffer.is_full()
    }

    pub fn produce<F>(&self, min_available: usize, f: F)
    where
        F: FnOnce(&mut [T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        assert!(
            min_available <= buffer.capacity(),
            "Requested minimum available items exceeds buffer capacity",
        );

        // Wait until there is enough space to write
        while buffer.free() < min_available {
            condvar.wait(&mut buffer);
        }

        // Now this slice will have more or equal to `min_available` items available
        let write_slice = buffer.as_mut_slice();

        // Closure writes into it and returns how many items were written
        let produced = f(write_slice);

        // Advance the write pointer
        buffer.produce(produced);

        // Notify any waiting consumers
        condvar.notify_one();
    }
}

// Consumer part
pub struct Consumer<T> {
    shared: Arc<Shared<T>>,
}

impl<T: Copy> Consumer<T> {
    pub fn is_empty(&self) -> bool {
        let buffer = self.shared.buffer.lock();
        buffer.is_empty()
    }

    /// Consume data by giving a slice of readable items to the closure.
    /// The closure returns how many items it actually consumed.
    pub fn consume<F>(&self, min_available: usize, f: F)
    where
        F: FnOnce(&[T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        assert!(
            min_available <= buffer.capacity(),
            "Requested minimum available items exceeds buffer capacity",
        );

        // Wait until there is at least one item to read
        while buffer.len() < min_available {
            condvar.wait(&mut buffer);
        }

        let read_slice = buffer.as_slice(); // Get readable slice

        let consumed = f(read_slice); // Closure consumes items

        buffer.consume(consumed); // Advance read position

        // Notify any waiting producers
        condvar.notify_one();
    }
}

pub fn ring_buffer_pair<T: Copy>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    let shared = Arc::new(Shared {
        buffer: Mutex::new(RingBuffer::new(capacity)),
        condvar: Condvar::new(),
    });

    (
        Producer {
            shared: Arc::clone(&shared),
        },
        Consumer { shared },
    )
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        // placeholder
        let capacity = 1024;

        let test_rounds = 13489;
        let produce_num = 101;

        let (producer, consumer) = ring_buffer_pair::<u8>(capacity);

        // Thread 0
        let producer_thread = std::thread::spawn(move || {
            assert!(!producer.is_full());

            for _ in 0..test_rounds {
                // Produce some data
                producer.produce(produce_num, |slice| {
                    // Fill the slice with some data

                    for i in 0..slice.len() {
                        slice[i] = (i % 256) as u8;
                    }
                    slice.len() // return how many items were produced
                });
            }
        });

        // Consumer thread
        let consumer_thread = std::thread::spawn(move || {
            for _ in 0..test_rounds {
                consumer.consume(produce_num, |slice| {
                    // Consume the slice and check the data
                    for i in 0..slice.len() {
                        assert_eq!(slice[i], (i % 256) as u8);
                    }
                    slice.len() // return how many items were consumed
                });
            }
        });

        // join
        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }
}
