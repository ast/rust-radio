use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use crossbeam::channel::bounded;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct ChannelPool<T: Default + Clone> {
    //pool: Mutex<Vec<Arc<Vec<T>>>>,
    receiver: Receiver<Arc<Vec<T>>>,
    sender: Sender<Arc<Vec<T>>>,
}

impl<T: Default + Clone> ChannelPool<T> {
    pub fn new(capacity: usize, buffer_len: usize) -> Self {
        // Create a bounded channel for the buffer pool
        let (sender, receiver) = bounded(capacity);
        // Push initial buffers into the channel
        for _ in 0..capacity {
            let buffer = Arc::new(vec![T::default(); buffer_len]);
            sender
                .send(buffer)
                .expect("Failed to send buffer to channel");
        }

        Self { receiver, sender }
    }

    pub fn get(&self) -> BufferGuard<T> {
        let buffer = self.receiver.recv().expect("Buffer pool is empty!!!");

        BufferGuard {
            buffer: Some(buffer),
            sender: self.sender.clone(),
        }
    }
}

#[derive(Clone)]
pub struct BufferGuard<T: Default + Clone> {
    buffer: Option<Arc<Vec<T>>>,
    sender: Sender<Arc<Vec<T>>>,
}

impl<T: Default + Clone> BufferGuard<T> {
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(self.buffer.as_ref().unwrap())
    }
}

impl<T: Default + Clone> Drop for BufferGuard<T> {
    fn drop(&mut self) {
        // Take buffer, leaving None in self
        let buf = self.buffer.take().expect("Buffer already taken!");

        // If this is the last reference to the buffer, return it to the pool
        if Arc::strong_count(&buf) == 1 {
            self.sender
                .send(buf)
                .expect("Failed to return buffer to channel");
        }
    }
}

impl<T: Default + Clone> Deref for BufferGuard<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}

impl<T: Default + Clone> DerefMut for BufferGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::get_mut(self.buffer.as_mut().unwrap())
            .expect("multiple references exist")
            .as_mut_slice()
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    use crossbeam::channel::bounded;

    #[test]
    fn test_channel_pool() {
        let pool = ChannelPool::<f32>::new(10, 1024);
        let guard = pool.get();
        assert_eq!(guard.ref_count(), 1);

        // Drop the guard and check if the buffer is returned to the pool
        drop(guard);
        let guard2 = pool.get();
        assert_eq!(guard2.ref_count(), 1);
    }

    // Test send to thread
    #[test]
    // #[ignore] // This test is ignored because it requires a multi-threaded environment
    fn test_channel_pool_send() {
        let pool = ChannelPool::<f32>::new(10, 32768);

        // A channel for sending buffers
        let (tx, rx) = bounded(11);

        std::thread::scope(|s| {
            // Sender
            s.spawn(|| {
                for _ in 0..1000 {
                    let mut buffer_guard = pool.get();
                    // Fill the buffer with some data
                    for j in 0..buffer_guard.len() {
                        buffer_guard[j] = j as f32
                    }
                    // Send the buffer to the channel
                    tx.send(buffer_guard).expect("Failed to send buffer");
                }
            });

            // Receiver
            s.spawn(|| {
                for _ in 0..1000 {
                    let buffer_guard = rx.recv().expect("Failed to receive buffer");
                    // Check the data in the buffer
                    for j in 0..buffer_guard.len() {
                        assert_eq!(buffer_guard[j], j as f32);
                    }
                }
            });
        });
    }
}
