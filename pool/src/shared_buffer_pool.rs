use parking_lot::{Condvar, Mutex};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};

/// A pool of reusable buffers that supports a broadcast fan-out pattern:
/// producer checks out a unique mutable handle, fills it, then `freeze`s it
/// into a cheap-to-clone shared read-only handle. Each broadcast receiver
/// clones a `Shared`; the underlying storage is returned to the pool only
/// when the last `Shared` is dropped.
pub struct SharedBufferPool<T: Default + Clone> {
    pool: Mutex<Vec<Vec<T>>>,
    condvar: Condvar,
}

struct Inner<T: Default + Clone> {
    buf: ManuallyDrop<Vec<T>>,
    pool: Weak<SharedBufferPool<T>>,
}

impl<T: Default + Clone> Drop for Inner<T> {
    fn drop(&mut self) {
        // SAFETY: Inner is dropped at most once (by Arc), and `buf` is
        // only taken here.
        let buf = unsafe { ManuallyDrop::take(&mut self.buf) };
        if let Some(pool) = self.pool.upgrade() {
            pool.put(buf);
        }
        // If the pool is gone, `buf` drops normally and frees the Vec.
    }
}

impl<T: Default + Clone> SharedBufferPool<T> {
    pub fn new(capacity: usize, buffer_len: usize) -> Arc<Self> {
        let mut buffers = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffers.push(vec![T::default(); buffer_len]);
        }
        Arc::new(Self {
            pool: Mutex::new(buffers),
            condvar: Condvar::new(),
        })
    }

    /// Block until a buffer is available, then return a unique mutable handle.
    pub fn get(self: &Arc<Self>) -> BufferMut<T> {
        let mut pool = self.pool.lock();
        self.condvar.wait_while(&mut pool, |p| p.is_empty());
        let buf = pool.pop().expect("condvar woke but pool is empty");
        BufferMut {
            inner: Arc::new(Inner {
                buf: ManuallyDrop::new(buf),
                pool: Arc::downgrade(self),
            }),
        }
    }

    /// Non-blocking variant. Returns `None` when all buffers are outstanding
    /// so the caller can drop the frame instead of blocking.
    pub fn try_get(self: &Arc<Self>) -> Option<BufferMut<T>> {
        let mut pool = self.pool.lock();
        let buf = pool.pop()?;
        Some(BufferMut {
            inner: Arc::new(Inner {
                buf: ManuallyDrop::new(buf),
                pool: Arc::downgrade(self),
            }),
        })
    }

    fn put(&self, buf: Vec<T>) {
        let mut pool = self.pool.lock();
        pool.push(buf);
        self.condvar.notify_one();
    }
}

/// Unique, mutable handle to a pooled buffer.
pub struct BufferMut<T: Default + Clone> {
    inner: Arc<Inner<T>>,
}

impl<T: Default + Clone> BufferMut<T> {
    /// Convert into a cloneable read-only handle. The buffer returns to the
    /// pool when the last `Shared` clone is dropped.
    pub fn freeze(self) -> Shared<T> {
        Shared { inner: self.inner }
    }
}

impl<T: Default + Clone> Deref for BufferMut<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.inner.buf
    }
}

impl<T: Default + Clone> DerefMut for BufferMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // We are the sole holder of this Arc<Inner>; safe to get_mut.
        let inner = Arc::get_mut(&mut self.inner)
            .expect("BufferMut::deref_mut: Inner has outstanding clones");
        &mut inner.buf
    }
}

/// Cloneable, read-only handle. Last drop returns the buffer to the pool.
pub struct Shared<T: Default + Clone> {
    inner: Arc<Inner<T>>,
}

impl<T: Default + Clone> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared { inner: Arc::clone(&self.inner) }
    }
}

impl<T: Default + Clone> Deref for Shared<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.inner.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::thread;

    #[test]
    fn test_get_freeze_reuse() {
        const BUF_SIZE: usize = 32;
        let pool = SharedBufferPool::<u8>::new(2, BUF_SIZE);

        let ptr0 = {
            let mut buf = pool.get();
            assert_eq!(buf.len(), BUF_SIZE);
            buf[0] = 42;
            let ptr = buf.as_ptr();
            let s = buf.freeze();
            assert_eq!(s[0], 42);
            drop(s);
            ptr
        };

        // Fetch repeatedly until we see the same allocation again; the pool
        // has 2 buffers so at most 2 gets are needed.
        let mut reused = false;
        for _ in 0..2 {
            let buf = pool.get();
            if buf.as_ptr() == ptr0 {
                reused = true;
            }
        }
        assert!(reused, "expected buffer allocation to be reused");
    }

    #[test]
    fn test_threaded_producer_consumer() {
        const BUF_SIZE: usize = 2048;
        const POOL_SIZE: usize = 4;
        const NUM_MESSAGES: usize = 100;

        thread::scope(|s| {
            let pool = SharedBufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);
            let (tx, rx) = channel::<Shared<u32>>();

            s.spawn(move || {
                for i in 0..NUM_MESSAGES {
                    let mut buf = pool.get();
                    for j in 0..BUF_SIZE {
                        buf[j] = j as u32 + i as u32 * BUF_SIZE as u32;
                    }
                    tx.send(buf.freeze()).unwrap();
                }
            });

            s.spawn(move || {
                for i in 0..NUM_MESSAGES {
                    let buf = rx.recv().unwrap();
                    assert_eq!(buf.len(), BUF_SIZE);
                    assert_eq!(buf[0], i as u32 * BUF_SIZE as u32);
                }
            });
        });
    }

    #[test]
    fn test_exhaustion_shared_blocks() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        let pool = SharedBufferPool::<u8>::new(1, 64);
        let unblocked = Arc::new(AtomicBool::new(false));

        let shared = pool.get().freeze();
        let shared_clone = shared.clone();

        let unblocked2 = unblocked.clone();
        let pool2 = Arc::clone(&pool);
        let handle = thread::spawn(move || {
            let _buf = pool2.get();
            unblocked2.store(true, Ordering::SeqCst);
        });

        thread::sleep(Duration::from_millis(50));
        assert!(!unblocked.load(Ordering::SeqCst), "should still be blocked");

        // Dropping one of two Shared clones does NOT return the buffer.
        drop(shared);
        thread::sleep(Duration::from_millis(50));
        assert!(
            !unblocked.load(Ordering::SeqCst),
            "should still block while one Shared clone is alive"
        );

        // Dropping the last Shared clone returns the buffer.
        drop(shared_clone);
        handle.join().unwrap();
        assert!(unblocked.load(Ordering::SeqCst), "should have unblocked");
    }

    #[test]
    fn test_buffermut_dropped_without_freeze_returns() {
        let pool = SharedBufferPool::<u8>::new(1, 16);
        let buf = pool.get();
        drop(buf);
        // Should not block.
        let _buf2 = pool.get();
    }

    #[test]
    fn test_concurrent_get_freeze_drop() {
        const POOL_SIZE: usize = 4;
        const ROUNDS: usize = 500;

        let pool = SharedBufferPool::<u8>::new(POOL_SIZE, 128);

        thread::scope(|s| {
            for _ in 0..8 {
                let pool = Arc::clone(&pool);
                s.spawn(move || {
                    for _ in 0..ROUNDS {
                        let mut buf = pool.get();
                        buf[0] = 42;
                        let s = buf.freeze();
                        // Fan out a few clones and drop them.
                        let c1 = s.clone();
                        let c2 = s.clone();
                        drop(c1);
                        drop(c2);
                        drop(s);
                    }
                });
            }
        });

        // All buffers should be back in the pool.
        let mut guards = Vec::new();
        for _ in 0..POOL_SIZE {
            guards.push(pool.get());
        }
        assert_eq!(guards.len(), POOL_SIZE);
    }

    #[test]
    fn test_pool_dropped_while_shared_outstanding() {
        let pool = SharedBufferPool::<u8>::new(1, 16);
        let shared = pool.get().freeze();
        drop(pool);
        // Accessing the buffer must still be valid.
        assert_eq!(shared.len(), 16);
        drop(shared);
    }
}
