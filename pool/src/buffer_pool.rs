use parking_lot::{Condvar, Mutex};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct BufferPool<T: Default + Clone> {
    pool: Mutex<Vec<Arc<Vec<T>>>>,
    condvar: Condvar,
}

impl<T: Default + Clone> BufferPool<T> {
    pub fn new(capacity: usize, buffer_len: usize) -> Arc<Self> {
        let mut buffers = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            buffers.push(Arc::new(vec![T::default(); buffer_len]));
        }

        Arc::new(Self {
            pool: Mutex::new(buffers),
            condvar: Condvar::new(),
        })
    }

    pub fn get(self: &Arc<Self>) -> BufferGuard<T> {
        let mut pool = self.pool.lock();
        // Wait until there is a buffer available
        self.condvar.wait_while(&mut pool, |pool| pool.is_empty());
        let buffer = pool.pop().expect("condvar woke but pool is empty");
        BufferGuard {
            buffer: Some(buffer),
            pool: Arc::clone(self),
        }
    }

    /// Non-blocking variant. Returns `None` when the pool is empty so the
    /// caller can skip the allocation (e.g. drop a frame) instead of stalling.
    pub fn try_get(self: &Arc<Self>) -> Option<BufferGuard<T>> {
        let mut pool = self.pool.lock();
        let buffer = pool.pop()?;
        Some(BufferGuard {
            buffer: Some(buffer),
            pool: Arc::clone(self),
        })
    }

    fn put(&self, buffer: Arc<Vec<T>>) {
        let mut pool = self.pool.lock();

        debug_assert_eq!(
            Arc::strong_count(&buffer),
            1,
            "buffer has more than one reference on return to pool"
        );

        pool.push(buffer);

        // Notify one waiting thread that a buffer is available
        self.condvar.notify_one();
    }
}

pub struct BufferGuard<T: Default + Clone> {
    // Option so we can take it in Drop and leave None behind
    buffer: Option<Arc<Vec<T>>>,
    pool: Arc<BufferPool<T>>,
}

impl<T: Default + Clone> Drop for BufferGuard<T> {
    fn drop(&mut self) {
        let buf = self.buffer.take().expect("buffer already taken");
        self.pool.put(buf);
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
    use std::sync::mpsc::channel;
    use std::thread;

    #[test]
    fn test_arc_buffer_pool() {
        const BUF_SIZE: usize = 32768;

        let pool = BufferPool::<f32>::new(10, BUF_SIZE);
        {
            let mut buf1 = pool.get();
            assert_eq!(buf1.len(), BUF_SIZE);

            // fill the buffer with some data
            for i in 0..BUF_SIZE {
                buf1[i] = i as f32;
            }

            // check the data
            for i in 0..BUF_SIZE {
                assert_eq!(buf1[i], i as f32);
            }
        } // buffer is dropped and returned to pool

        {
            let mut buf2 = pool.get();

            buf2[0] = 123.0; // reuse the buffer

            assert_eq!(buf2.len(), BUF_SIZE);
            assert_eq!(buf2[0], 123.0); // reused buffer!
        }
    }

    #[test]
    fn test_threaded_buffer_reuse() {
        const BUF_SIZE: usize = 2048;
        const POOL_SIZE: usize = 10;
        const NUM_MESSAGES: usize = 100;

        thread::scope(|s| {
            let pool = BufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);
            let (tx, rx) = channel();

            s.spawn(move || {
                for i in 0..NUM_MESSAGES {
                    let mut buf = pool.get();

                    for j in 0..BUF_SIZE {
                        buf[j] = j as u32 + i as u32 * BUF_SIZE as u32;
                    }

                    tx.send(buf).unwrap();
                }
            });

            s.spawn(move || {
                for _ in 0..NUM_MESSAGES {
                    let buf = rx.recv().unwrap();
                    assert_eq!(buf.len(), BUF_SIZE);
                }
            });
        });
    }

    #[test]
    fn test_pool_exhaustion_blocks() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        let pool = BufferPool::<u8>::new(1, 64);
        let unblocked = Arc::new(AtomicBool::new(false));

        // Take the only buffer
        let buf = pool.get();

        let unblocked2 = unblocked.clone();
        let pool2 = Arc::clone(&pool);
        let handle = thread::spawn(move || {
            // This should block until we drop buf
            let _buf2 = pool2.get();
            unblocked2.store(true, Ordering::SeqCst);
        });

        // Give the other thread time to block
        thread::sleep(Duration::from_millis(50));
        assert!(!unblocked.load(Ordering::SeqCst), "should still be blocked");

        // Release — other thread should unblock
        drop(buf);
        handle.join().unwrap();
        assert!(unblocked.load(Ordering::SeqCst), "should have unblocked");
    }

    #[test]
    fn test_concurrent_get_return() {
        const POOL_SIZE: usize = 4;
        const ROUNDS: usize = 500;

        let pool = BufferPool::<u8>::new(POOL_SIZE, 128);

        thread::scope(|s| {
            for _ in 0..8 {
                let pool = Arc::clone(&pool);
                s.spawn(move || {
                    for _ in 0..ROUNDS {
                        let mut buf = pool.get();
                        buf[0] = 42;
                        // buf returned on drop
                    }
                });
            }
        });

        // All buffers should be back in the pool
        // We can get POOL_SIZE buffers without blocking
        let mut guards = Vec::new();
        for _ in 0..POOL_SIZE {
            guards.push(pool.get());
        }
        assert_eq!(guards.len(), POOL_SIZE);
    }
}
