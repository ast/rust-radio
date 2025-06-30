use super::Doublemap;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;

pub struct ProduceError;
pub struct ConsumeError;

unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Sync> Sync for RingBuffer<T> {}

// Inner ring buffer
#[derive(Debug)]
struct RingBuffer<T> {
    buffer: Doublemap<T>, // delay buffer
    capacity: usize,      // capacity of the buffer (must be a power of two)
    write: usize,         // write position
    read: usize,          // read position
    consumer_alive: bool, // consumer alive flag
    producer_alive: bool, // producer alive flag
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
            consumer_alive: true,
            producer_alive: true,
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

    fn consume(&mut self, num: usize) {
        assert!(num <= self.len(), "Cannot consume more than available");
        self.read = self.read.wrapping_add(num);
    }

    // A slice of the readable part
    fn as_slice(&self) -> &[T] {
        let start = self.mask(self.read);
        unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().add(start), self.len()) }
    }

    // A mut slice of the writable part
    fn as_mut_slice(&mut self) -> &mut [T] {
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
    required: usize,          // How many items the producer requires to proceed
    consumer_required: usize, // How many items the consumer requires to proceed
}

impl<T: Copy> Producer<T> {
    pub fn is_full(&self) -> bool {
        let buffer = self.shared.buffer.lock();
        buffer.is_full()
    }

    pub fn produce<F>(&self, f: F)
    where
        F: FnOnce(&mut [T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        condvar.wait_while(&mut buffer, |b| b.free() < self.required);

        let produced = {
            let write_slice = buffer.as_mut_slice();
            f(write_slice)
        };

        // Advance the write pointer
        buffer.produce(produced);

        if buffer.len() >= self.consumer_required {
            // If the length is enough for the consumer's requirement, notify
            condvar.notify_one();
        }
    }
}

impl<T> Drop for Producer<T> {
    fn drop(&mut self) {
        // Set the producer alive flag to false
        let mut buffer = self.shared.buffer.lock();
        buffer.producer_alive = false;
        // Notify any waiting consumers
        self.shared.condvar.notify_all();
    }
}

// Consumer part
pub struct Consumer<T> {
    shared: Arc<Shared<T>>,
    required: usize,          // How many items the consumer requires to proceed
    producer_required: usize, // How many items the producer requires to proceed
}

impl<T: Copy> Consumer<T> {
    pub fn is_empty(&self) -> bool {
        let buffer = self.shared.buffer.lock();
        buffer.is_empty()
    }

    /// Consume data by giving a slice of readable items to the closure.
    /// The closure returns how many items it actually consumed.
    pub fn consume<F>(&self, f: F)
    where
        F: FnOnce(&[T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        condvar.wait_while(&mut buffer, |b| b.len() < self.required);

        let consumed = {
            let read_slice = buffer.as_slice();
            f(read_slice)
        };

        buffer.consume(consumed); // Advance read position

        // Notify any waiting producers

        if buffer.free() >= self.producer_required {
            // If the free space is enough for the producer's requirement, notify
            condvar.notify_one();
        }
    }
}

impl<T> Drop for Consumer<T> {
    fn drop(&mut self) {
        // Set the consumer alive flag to false
        let mut buffer = self.shared.buffer.lock();
        buffer.consumer_alive = false;
        // Notify any waiting producers
        self.shared.condvar.notify_all();
    }
}

pub fn ring_buffer_pair<T: Copy>(
    producer_required: usize,
    consumer_required: usize,
) -> (Producer<T>, Consumer<T>) {
    // Estimate required capacity by doubling the maximum of producer and consumer requirements
    // 8 is just a heuristic to ensure we have enough space for both producer and consumer
    let capacity = 8 * producer_required.max(consumer_required).next_power_of_two();

    let shared = Arc::new(Shared {
        buffer: Mutex::new(RingBuffer::new(capacity)),
        condvar: Condvar::new(),
    });

    let producer = Producer {
        shared: Arc::clone(&shared),
        required: producer_required,
        consumer_required,
    };

    let consumer = Consumer {
        shared,
        required: consumer_required,
        producer_required,
    };

    (producer, consumer)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let test_rounds = 13489;

        let (producer, consumer) = ring_buffer_pair::<u8>(101, 101);

        // Thread 0
        let producer_thread = std::thread::spawn(move || {
            assert!(!producer.is_full());

            for _ in 0..test_rounds {
                // Produce some data
                producer.produce(|slice| {
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
                consumer.consume(|slice| {
                    // Consume the slice and check the data
                    (0..slice.len()).for_each(|i| {
                        assert_eq!(slice[i], (i % 256) as u8);
                    });
                    slice.len() // return how many items were consumed
                });
            }
        });

        // join
        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }
}
