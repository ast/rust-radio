use super::Doublemap;
use crossbeam_utils::CachePadded;
use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// Inner ring buffer
#[derive(Debug)]
struct AtomicRingBuffer<T> {
    buffer: Doublemap<T>,            // delay buffer
    write: CachePadded<AtomicUsize>, // written only by producer
    read: CachePadded<AtomicUsize>,  // written only by consumer
}

impl<T: Copy> AtomicRingBuffer<T> {
    fn new(capacity: usize) -> Self {
        AtomicRingBuffer {
            buffer: Doublemap::new(capacity).expect("Failed to create Doublemap"),
            write: CachePadded::new(AtomicUsize::new(0)),
            read: CachePadded::new(AtomicUsize::new(0)),
        }
    }

    fn mask(&self, index: usize) -> usize {
        index & (self.buffer.len() - 1)
    }

    fn len(&self) -> usize {
        let write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Acquire);
        write.wrapping_sub(read)
    }

    fn capacity(&self) -> usize {
        self.buffer.len()
    }

    fn available(&self) -> usize {
        self.capacity() - self.len()
    }

    fn as_slice(&self) -> &[T] {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let len = write.wrapping_sub(read);
        let start = self.mask(read);
        let end = start + len;
        &self.buffer.as_slice()[start..end]
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let available = self.buffer.len() - (write.wrapping_sub(read));
        let start = self.mask(write);
        let end = start + available;
        &mut self.buffer.as_mut_slice()[start..end]
    }

    fn produce(&self, num: usize) {
        let write = self.write.load(Ordering::Relaxed);
        self.write.store(write.wrapping_add(num), Ordering::Release);
    }

    fn consume(&self, num: usize) {
        let read = self.read.load(Ordering::Relaxed);
        self.read.store(read.wrapping_add(num), Ordering::Release);
    }
}

struct Shared<T> {
    buffer: Mutex<AtomicRingBuffer<T>>,
    condvar: Condvar,
}

pub struct Producer<T> {
    shared: Arc<Shared<T>>,
}

impl<T: Copy> Producer<T> {
    pub fn produce<F>(&self, min_available: usize, f: F)
    where
        F: FnOnce(&mut [T]) -> usize,
    {
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        while buffer.available() < min_available {
            condvar.wait(&mut buffer);
        }

        let slice = buffer.as_mut_slice();
        let produced = f(slice);
        buffer.produce(produced);
        condvar.notify_one();
    }
}

pub struct Consumer<T> {
    shared: Arc<Shared<T>>,
}

impl<T: Copy> Consumer<T> {
    pub fn consume<F>(&self, min_available: usize, f: F)
    where
        F: FnOnce(&[T]) -> usize,
    {
        let Shared { buffer, condvar } = &*self.shared;
        let mut buffer = buffer.lock();

        while buffer.len() < min_available {
            condvar.wait(&mut buffer);
        }

        let slice = buffer.as_slice();
        let consumed = f(slice);
        buffer.consume(consumed);
        condvar.notify_one();
    }
}

pub fn atomic_ring_buffer_pair<T: Copy>(capacity: usize) -> (Producer<T>, Consumer<T>) {
    let shared = Arc::new(Shared {
        buffer: Mutex::new(AtomicRingBuffer::new(capacity)),
        condvar: Condvar::new(),
    });

    (
        Producer {
            shared: Arc::clone(&shared),
        },
        Consumer { shared },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer() {
        let capacity = 1024;
        let test_rounds = 13489;
        let produce_num = 101;

        let (producer, consumer) = atomic_ring_buffer_pair::<u8>(capacity);

        let producer_thread = std::thread::spawn(move || {
            for _ in 0..test_rounds {
                producer.produce(produce_num, |slice| {
                    for i in 0..slice.len() {
                        slice[i] = (i % 256) as u8;
                    }
                    slice.len()
                });
            }
        });

        let consumer_thread = std::thread::spawn(move || {
            for _ in 0..test_rounds {
                consumer.consume(produce_num, |slice| {
                    for i in 0..slice.len() {
                        assert_eq!(slice[i], (i % 256) as u8);
                    }
                    slice.len()
                });
            }
        });

        producer_thread.join().unwrap();
        consumer_thread.join().unwrap();
    }
}
