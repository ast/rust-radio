use pool::{BufferPool, ChannelPool};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Simulate an SDR pipeline: one producer fills buffers at a steady rate,
/// multiple consumers process them. Verify every sample is correct.
#[test]
fn test_pipeline_data_integrity_buffer_pool() {
    const BUF_SIZE: usize = 2048;
    const POOL_SIZE: usize = 8;
    const NUM_BUFFERS: usize = 500;

    let pool = BufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);
    let (tx, rx) = channel();

    thread::scope(|s| {
        // Producer: fill each buffer with a unique sequence
        let pool = Arc::clone(&pool);
        s.spawn(move || {
            for i in 0..NUM_BUFFERS {
                let mut buf = pool.get();
                let base = (i * BUF_SIZE) as u32;
                for j in 0..BUF_SIZE {
                    buf[j] = base + j as u32;
                }
                tx.send(buf).unwrap();
            }
        });

        // Consumer: verify every value
        s.spawn(move || {
            for i in 0..NUM_BUFFERS {
                let buf = rx.recv().unwrap();
                let base = (i * BUF_SIZE) as u32;
                for j in 0..BUF_SIZE {
                    assert_eq!(buf[j], base + j as u32, "mismatch at buffer {i} index {j}");
                }
            }
        });
    });
}

/// Same pipeline test but with ChannelPool.
#[test]
fn test_pipeline_data_integrity_channel_pool() {
    const BUF_SIZE: usize = 2048;
    const POOL_SIZE: usize = 8;
    const NUM_BUFFERS: usize = 500;

    let pool = ChannelPool::<u32>::new(POOL_SIZE, BUF_SIZE);
    let (tx, rx) = channel();

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..NUM_BUFFERS {
                let mut buf = pool.get();
                let base = (i * BUF_SIZE) as u32;
                for j in 0..BUF_SIZE {
                    buf[j] = base + j as u32;
                }
                tx.send(buf).unwrap();
            }
        });

        s.spawn(move || {
            for i in 0..NUM_BUFFERS {
                let buf = rx.recv().unwrap();
                let base = (i * BUF_SIZE) as u32;
                for j in 0..BUF_SIZE {
                    assert_eq!(buf[j], base + j as u32, "mismatch at buffer {i} index {j}");
                }
            }
        });
    });
}

/// Multiple producers, one consumer. All buffers come from the same pool.
/// Verifies no corruption when multiple threads borrow concurrently.
#[test]
fn test_multi_producer_single_consumer() {
    const BUF_SIZE: usize = 1024;
    const POOL_SIZE: usize = 6;
    const PRODUCERS: usize = 4;
    const BUFFERS_PER_PRODUCER: usize = 200;

    let pool = BufferPool::<u64>::new(POOL_SIZE, BUF_SIZE);
    let (tx, rx) = channel();
    let total = PRODUCERS * BUFFERS_PER_PRODUCER;

    thread::scope(|s| {
        for producer_id in 0..PRODUCERS {
            let pool = Arc::clone(&pool);
            let tx = tx.clone();
            s.spawn(move || {
                for seq in 0..BUFFERS_PER_PRODUCER {
                    let mut buf = pool.get();
                    // Tag every element with producer_id and sequence
                    let tag = ((producer_id as u64) << 32) | seq as u64;
                    for j in 0..BUF_SIZE {
                        buf[j] = tag;
                    }
                    tx.send((producer_id, seq, buf)).unwrap();
                }
            });
        }
        drop(tx); // close sender so consumer loop ends

        s.spawn(move || {
            let mut count = 0;
            for (pid, seq, buf) in rx {
                let expected = ((pid as u64) << 32) | seq as u64;
                for j in 0..BUF_SIZE {
                    assert_eq!(buf[j], expected, "corruption from producer {pid} seq {seq} at {j}");
                }
                count += 1;
            }
            assert_eq!(count, total);
        });
    });
}

/// Hammer a tiny pool from many threads to stress the blocking path.
/// Counts total successful get/return cycles.
#[test]
fn test_contention_stress() {
    const POOL_SIZE: usize = 2;
    const THREADS: usize = 16;
    const OPS_PER_THREAD: usize = 1000;

    let pool = BufferPool::<u8>::new(POOL_SIZE, 64);
    let counter = Arc::new(AtomicU64::new(0));

    thread::scope(|s| {
        for _ in 0..THREADS {
            let pool = Arc::clone(&pool);
            let counter = Arc::clone(&counter);
            s.spawn(move || {
                for _ in 0..OPS_PER_THREAD {
                    let mut buf = pool.get();
                    // Do a small amount of work while holding the buffer
                    buf[0] = buf[0].wrapping_add(1);
                    counter.fetch_add(1, Ordering::Relaxed);
                    // returned on drop
                }
            });
        }
    });

    assert_eq!(
        counter.load(Ordering::SeqCst),
        (THREADS * OPS_PER_THREAD) as u64
    );

    // All buffers should be back
    for _ in 0..POOL_SIZE {
        let _buf = pool.get();
    }
}

/// Same contention test for ChannelPool.
#[test]
fn test_contention_stress_channel_pool() {
    const POOL_SIZE: usize = 2;
    const THREADS: usize = 16;
    const OPS_PER_THREAD: usize = 1000;

    let pool = Arc::new(ChannelPool::<u8>::new(POOL_SIZE, 64));
    let counter = Arc::new(AtomicU64::new(0));

    thread::scope(|s| {
        for _ in 0..THREADS {
            let pool = Arc::clone(&pool);
            let counter = Arc::clone(&counter);
            s.spawn(move || {
                for _ in 0..OPS_PER_THREAD {
                    let mut buf = pool.get();
                    buf[0] = buf[0].wrapping_add(1);
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });

    assert_eq!(
        counter.load(Ordering::SeqCst),
        (THREADS * OPS_PER_THREAD) as u64
    );
}

/// Verify that buffers passed through a channel maintain their data
/// even when the pool is under pressure (pool smaller than channel depth).
#[test]
fn test_in_flight_buffers_preserved() {
    const BUF_SIZE: usize = 512;
    const POOL_SIZE: usize = 4;
    const ROUNDS: usize = 300;

    let pool = BufferPool::<f32>::new(POOL_SIZE, BUF_SIZE);
    // Channel can hold more than pool size — buffers are in-flight
    let (tx, rx) = channel();

    thread::scope(|s| {
        let pool = Arc::clone(&pool);
        s.spawn(move || {
            for i in 0..ROUNDS {
                let mut buf = pool.get();
                let val = i as f32;
                for j in 0..BUF_SIZE {
                    buf[j] = val;
                }
                tx.send((i, buf)).unwrap();
            }
        });

        s.spawn(move || {
            for _ in 0..ROUNDS {
                let (i, buf) = rx.recv().unwrap();
                let expected = i as f32;
                // Spot-check several positions
                assert_eq!(buf[0], expected);
                assert_eq!(buf[BUF_SIZE / 2], expected);
                assert_eq!(buf[BUF_SIZE - 1], expected);
            }
        });
    });
}

/// Verify that pool works correctly when buffers are held for varying
/// durations (simulates uneven processing times).
#[test]
fn test_variable_hold_times() {
    const BUF_SIZE: usize = 256;
    const POOL_SIZE: usize = 3;
    const THREADS: usize = 6;
    const OPS: usize = 50;

    let pool = BufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);
    let completed = Arc::new(AtomicU64::new(0));

    thread::scope(|s| {
        for t in 0..THREADS {
            let pool = Arc::clone(&pool);
            let completed = Arc::clone(&completed);
            s.spawn(move || {
                for i in 0..OPS {
                    let mut buf = pool.get();
                    let tag = (t * OPS + i) as u32;
                    for j in 0..BUF_SIZE {
                        buf[j] = tag;
                    }

                    // Vary hold time: some threads hold longer
                    if i % 7 == 0 {
                        thread::sleep(Duration::from_micros(100));
                    }

                    // Verify data is still ours
                    for j in 0..BUF_SIZE {
                        assert_eq!(buf[j], tag, "data corrupted for thread {t} op {i} at {j}");
                    }

                    completed.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });

    assert_eq!(
        completed.load(Ordering::SeqCst),
        (THREADS * OPS) as u64
    );
}
