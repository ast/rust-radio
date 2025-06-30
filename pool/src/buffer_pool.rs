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
        let buffer = pool.pop().expect("Buffer pool is empty!!!");
        BufferGuard {
            buffer: Some(buffer),
            pool: Arc::clone(self),
        }
    }

    pub fn put(&self, buffer: Arc<Vec<T>>) {
        let mut pool = self.pool.lock();

        assert_eq!(
            Arc::strong_count(&buffer),
            1,
            "Buffer has more than one reference!"
        );

        // There will always be space to put it back
        pool.push(buffer);

        // Notify one waiting thread that a buffer is available
        self.condvar.notify_one();
    }
}

#[derive(Clone)]
pub struct BufferGuard<T: Default + Clone> {
    // Buffer has to be an Option, so we can take it and leave None in
    // self and return it to the pool when dropped.
    buffer: Option<Arc<Vec<T>>>,
    pool: Arc<BufferPool<T>>,
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
            self.pool.put(buf);
        }
        // Else: someone else still holds a reference
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
    #[ignore] // This test is ignored because it requires a multi-threaded environment
    fn test_reuse() {
        let pool = BufferPool::<f32>::new(2, 8);
        {
            let mut buf = pool.get();

            buf[0] = 1.23;

            // clone the buffer
            let buf_clone = buf.clone();

            // print refcount of the buffer
            println!("Buffer refcount: {}", buf_clone.ref_count());

            assert_eq!(buf_clone[0], 1.23);
        } // buffer returned to pool

        {
            // get the buffer again
            let buf = pool.get();
            assert_eq!(buf[0], 0f32);
        }
    }

    #[test]
    fn test_threaded_buffer_reuse() {
        const BUF_SIZE: usize = 2048;
        const POOL_SIZE: usize = 10;
        const NUM_MESSAGES: usize = 10;

        thread::scope(|s| {
            let pool = BufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);
            // create channels
            let (tx, rx) = channel();

            s.spawn(move || {
                for i in 0..NUM_MESSAGES {
                    let mut buf = pool.get();

                    // mut slice from buf
                    for j in 0..BUF_SIZE {
                        buf[j] = j as u32 + i as u32 * BUF_SIZE as u32;
                    }

                    tx.send(buf).unwrap();
                    // sleep
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            });

            s.spawn(move || {
                let mut received = 0;

                for _ in 0..NUM_MESSAGES {
                    for _ in 0..10 {
                        match rx.recv() {
                            Ok(buf) => {
                                // check the buffer
                                assert_eq!(buf.len(), BUF_SIZE);
                                received += 1;
                            }
                            Err(e) => eprintln!("Failed to receive buffer: {}", e),
                        }
                    }
                }

                assert_eq!(received, 10);
            });
        });
    }
}
