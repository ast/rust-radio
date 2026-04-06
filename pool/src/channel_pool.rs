use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use crossbeam::channel::bounded;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct ChannelPool<T: Default + Clone> {
    receiver: Receiver<Arc<Vec<T>>>,
    sender: Sender<Arc<Vec<T>>>,
}

impl<T: Default + Clone> ChannelPool<T> {
    pub fn new(capacity: usize, buffer_len: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        for _ in 0..capacity {
            let buffer = Arc::new(vec![T::default(); buffer_len]);
            sender
                .send(buffer)
                .expect("channel should have capacity for initial buffers");
        }

        Self { receiver, sender }
    }

    pub fn get(&self) -> BufferGuard<T> {
        let buffer = self.receiver.recv().expect("pool channel disconnected");

        BufferGuard {
            buffer: Some(buffer),
            sender: self.sender.clone(),
        }
    }
}

pub struct BufferGuard<T: Default + Clone> {
    buffer: Option<Arc<Vec<T>>>,
    sender: Sender<Arc<Vec<T>>>,
}

impl<T: Default + Clone> Drop for BufferGuard<T> {
    fn drop(&mut self) {
        let buf = self.buffer.take().expect("buffer already taken");
        let _ = self.sender.send(buf);
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
            .expect("buffer has outstanding clones — cannot mutate")
            .as_mut_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crossbeam::channel::bounded;

    #[test]
    fn test_channel_pool() {
        let pool = ChannelPool::<f32>::new(10, 1024);
        let guard = pool.get();
        drop(guard);
        let _guard2 = pool.get();
    }

    #[test]
    fn test_channel_pool_send() {
        let pool = ChannelPool::<f32>::new(10, 32768);

        let (tx, rx) = bounded(11);

        std::thread::scope(|s| {
            s.spawn(|| {
                for _ in 0..1000 {
                    let mut buffer_guard = pool.get();
                    for j in 0..buffer_guard.len() {
                        buffer_guard[j] = j as f32
                    }
                    tx.send(buffer_guard).expect("Failed to send buffer");
                }
            });

            s.spawn(|| {
                for _ in 0..1000 {
                    let buffer_guard = rx.recv().expect("Failed to receive buffer");
                    for j in 0..buffer_guard.len() {
                        assert_eq!(buffer_guard[j], j as f32);
                    }
                }
            });
        });
    }
}
