use super::Doublemap;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("other side of ring buffer was dropped")]
pub struct Disconnected;

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

    pub fn produce<F>(&self, f: F) -> Result<(), Disconnected>
    where
        F: FnOnce(&mut [T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        // Wait until we have enough free space or the consumer is gone
        condvar.wait_while(&mut buffer, |b| {
            b.free() < self.required && b.consumer_alive
        });

        if !buffer.consumer_alive {
            return Err(Disconnected);
        }

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

        Ok(())
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
    pub fn consume<F>(&self, f: F) -> Result<(), Disconnected>
    where
        F: FnOnce(&[T]) -> usize,
    {
        // Borrow what's inside the Arc
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        // Wait until we have enough data or the producer is gone
        condvar.wait_while(&mut buffer, |b| {
            b.len() < self.required && b.producer_alive
        });

        // If the producer is gone and there's not enough data, signal disconnection
        if !buffer.producer_alive && buffer.len() < self.required {
            return Err(Disconnected);
        }

        let consumed = {
            let read_slice = buffer.as_slice();
            f(read_slice)
        };

        buffer.consume(consumed); // Advance read position

        if buffer.free() >= self.producer_required {
            // If the free space is enough for the producer's requirement, notify
            condvar.notify_one();
        }

        Ok(())
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
                producer
                    .produce(|slice| {
                        // Fill the slice with some data

                        (0..slice.len()).for_each(|i| {
                            slice[i] = (i % 256) as u8;
                        });

                        slice.len() // return how many items were produced
                    })
                    .unwrap();
            }
        });

        // Consumer thread
        let consumer_thread = std::thread::spawn(move || {
            for _ in 0..test_rounds {
                consumer
                    .consume(|slice| {
                        // Consume the slice and check the data
                        (0..slice.len()).for_each(|i| {
                            assert_eq!(slice[i], (i % 256) as u8);
                        });
                        slice.len() // return how many items were consumed
                    })
                    .unwrap();
            }
        });

        // join
        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }

    #[test]
    fn test_drop_producer_unblocks_consumer() {
        let (producer, consumer) = ring_buffer_pair::<u8>(64, 64);

        let handle = std::thread::spawn(move || {
            // Consumer waits for data that will never come
            let result = consumer.consume(|slice| slice.len());
            assert!(result.is_err(), "should return Disconnected");
        });

        // Drop the producer — consumer should wake up and get Err
        drop(producer);
        handle.join().unwrap();
    }

    #[test]
    fn test_drop_consumer_unblocks_producer() {
        let (producer, consumer) = ring_buffer_pair::<u8>(64, 64);

        // Fill the buffer completely so producer will block
        let cap = {
            let buf = producer.shared.buffer.lock();
            buf.capacity()
        };

        // Fill to capacity
        producer
            .produce(|slice| {
                let n = slice.len().min(cap);
                for i in 0..n {
                    slice[i] = 0;
                }
                n
            })
            .unwrap();

        let handle = std::thread::spawn(move || {
            // Producer tries to write into a full buffer
            let result = producer.produce(|slice| {
                slice[0] = 42;
                1
            });
            assert!(result.is_err(), "should return Disconnected");
        });

        // Drop the consumer — producer should wake up and get Err
        drop(consumer);
        handle.join().unwrap();
    }

    #[test]
    fn test_zst_rejected() {
        use crate::doublemap::DoublemapError;
        let result = super::super::Doublemap::<()>::new(1024);
        assert!(matches!(result, Err(DoublemapError::ZeroSizedType)));
    }

    #[test]
    fn test_wraparound_correctness() {
        // Verify data integrity across many wraparound cycles
        let chunk = 137; // odd size to stress wraparound
        let rounds = 500;

        let (producer, consumer) = ring_buffer_pair::<u32>(chunk, chunk);

        let producer_thread = std::thread::spawn(move || {
            for round in 0..rounds {
                producer
                    .produce(|slice| {
                        for i in 0..chunk {
                            slice[i] = (round * chunk + i) as u32;
                        }
                        chunk
                    })
                    .unwrap();
            }
        });

        let consumer_thread = std::thread::spawn(move || {
            let mut expected = 0u32;
            for _ in 0..rounds {
                consumer
                    .consume(|slice| {
                        for i in 0..chunk {
                            assert_eq!(
                                slice[i], expected,
                                "mismatch at expected={expected}, got={}",
                                slice[i]
                            );
                            expected += 1;
                        }
                        chunk
                    })
                    .unwrap();
            }
        });

        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }

    #[test]
    fn test_mirroring_across_boundary() {
        // Write at the end of the first half and verify it's visible
        // at the start of the second half (the core doublemap property)
        use crate::Doublemap;

        let mut map = Doublemap::<u64>::new(512).unwrap();
        let cap = map.capacity();
        let slice = map.as_mut_slice();

        // Write a sentinel at the last element of the first half
        let sentinel = 0xDEAD_BEEF_CAFE_BABEu64;
        slice[cap - 1] = sentinel;

        // It should appear at the mirror position
        assert_eq!(slice[cap - 1 + cap], sentinel);

        // Write at the start of the first half
        slice[0] = 0x1234;
        assert_eq!(slice[cap], 0x1234);
    }
}
