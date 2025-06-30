use criterion::{Criterion, criterion_group, criterion_main};

use pool::BufferPool;
use pool::ChannelPool;
use std::hint::black_box;
use std::sync::mpsc::channel;
use std::thread;

fn bench_buffer_reuse(c: &mut Criterion) {
    const BUF_SIZE: usize = 32768;
    const POOL_SIZE: usize = 100;
    const NUM_MESSAGES: usize = 1000;

    c.bench_function("channel buffer reuse", |b| {
        let pool = ChannelPool::<u32>::new(POOL_SIZE, BUF_SIZE);

        b.iter(|| {
            let (tx, rx) = channel();

            thread::scope(|s| {
                s.spawn({
                    || {
                        for i in 0..NUM_MESSAGES {
                            let mut buf = pool.get();
                            for j in 0..BUF_SIZE {
                                buf[j] = j as u32;
                            }
                            tx.send(buf).unwrap();
                        }
                    }
                });

                // Consumer
                s.spawn(move || {
                    let mut received = 0;
                    while received < NUM_MESSAGES {
                        if let Ok(buf) = rx.recv() {
                            black_box(&buf);
                            assert_eq!(buf.len(), BUF_SIZE);
                            received += 1;
                        }
                    }
                });
            });
        });
    });

    c.bench_function("threaded buffer reuse", |b| {
        let pool = BufferPool::<u32>::new(POOL_SIZE, BUF_SIZE);

        b.iter(|| {
            let (tx, rx) = channel();

            thread::scope(|s| {
                s.spawn({
                    || {
                        for i in 0..NUM_MESSAGES {
                            let mut buf = pool.get();
                            for j in 0..BUF_SIZE {
                                buf[j] = j as u32;
                            }
                            tx.send(buf).unwrap();
                        }
                    }
                });

                // Consumer
                s.spawn(move || {
                    let mut received = 0;
                    while received < NUM_MESSAGES {
                        if let Ok(buf) = rx.recv() {
                            black_box(&buf);
                            assert_eq!(buf.len(), BUF_SIZE);
                            received += 1;
                        }
                    }
                });
            });
        });
    });
}

criterion_group!(benches, bench_buffer_reuse);
criterion_main!(benches);
