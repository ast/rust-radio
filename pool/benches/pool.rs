use criterion::{Criterion, criterion_group, criterion_main};

// use num_complex::Complex32;
use pool::ArcBufferPool;
use std::hint::black_box;
use std::sync::mpsc::channel;
use std::thread;

pub fn threaded_buffer_roundtrip(pool_size: usize, buf_size: usize, num_messages: usize) {
    thread::scope(|s| {
        let pool = ArcBufferPool::<u32>::new(pool_size, buf_size);
        let (tx, rx) = channel();

        // Producer
        s.spawn({
            let tx = tx.clone();
            move || {
                for i in 0..num_messages {
                    let mut buf = pool.get();

                    for j in 0..buf_size {
                        buf[j] = j as u32 + i as u32 * buf_size as u32;
                    }

                    tx.send(buf).unwrap();
                    // No sleep in benchmarking version
                }
            }
        });

        // Drop extra sender
        drop(tx);

        // Consumer
        s.spawn(move || {
            let mut received = 0;
            while received < num_messages {
                if let Ok(buf) = rx.recv() {
                    assert_eq!(buf.len(), buf_size);
                    received += 1;
                }
            }
        });
    });
}

fn bench_buffer_reuse(c: &mut Criterion) {
    const BUF_SIZE: usize = 2048;
    const POOL_SIZE: usize = 100;
    const NUM_MESSAGES: usize = 1500;

    c.bench_function("threaded buffer reuse", |b| {
        b.iter(|| {
            black_box(threaded_buffer_roundtrip(POOL_SIZE, BUF_SIZE, NUM_MESSAGES));
        });
    });
}

criterion_group!(benches, bench_buffer_reuse);
criterion_main!(benches);
