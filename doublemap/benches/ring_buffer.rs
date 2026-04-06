use criterion::{Criterion, criterion_group, criterion_main};
use doublemap::thread_ring_buffer::{Consumer, Producer, ring_buffer_pair};
use num_complex::Complex32;
use std::hint::black_box;
use std::thread;

const NUM_SAMPLES: usize = 4 * 768_000;
const CHUNK_SIZE: usize = 2048;

fn bench_threaded_ring_buffer(c: &mut Criterion) {
    let input: Vec<Complex32> = (0..NUM_SAMPLES)
        .map(|i| Complex32::new(i as f32, -(i as f32)))
        .collect();

    let (producer, consumer): (Producer<Complex32>, Consumer<Complex32>) =
        ring_buffer_pair(CHUNK_SIZE, CHUNK_SIZE);

    c.bench_function("threaded ring buffer Complex32 768k", |b| {
        b.iter(|| {
            thread::scope(|s| {
                // Producer thread
                s.spawn(|| {
                    let mut index = 0;
                    while index < input.len() {
                        producer
                            .produce(|buf| {
                                let len = buf.len().min(CHUNK_SIZE).min(input.len() - index);
                                buf[..len].copy_from_slice(&input[index..index + len]);
                                index += len;
                                len
                            })
                            .unwrap();
                    }
                });

                // Consumer thread
                s.spawn(|| {
                    let mut received = 0;
                    while received < NUM_SAMPLES {
                        consumer
                            .consume(|buf| {
                                let len = buf.len().min(CHUNK_SIZE).min(NUM_SAMPLES - received);
                                black_box(&buf[..len]); // simulate processing
                                received += len;
                                len
                            })
                            .unwrap();
                    }
                });
            });
        });
    });
}

criterion_group!(benches, bench_threaded_ring_buffer,);
criterion_main!(benches);
