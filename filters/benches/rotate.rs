use criterion::{criterion_group, criterion_main, Criterion};
use filters::rotate::ComplexRotator; // adjust module path to your rotator
use num_complex::Complex32;
use std::f32::consts::PI;

use std::hint::black_box;

fn generate_complex_input(len: usize, sample_rate: f32, freq_hz: f32) -> Vec<Complex32> {
    let omega = 2.0 * PI * freq_hz / sample_rate;
    (0..len)
        .map(|i| {
            let phase = omega * i as f32;
            let (im, re) = phase.sin_cos();
            Complex32::new(re, im)
        })
        .collect()
}

fn bench_rotator_768ksps(c: &mut Criterion) {
    let sample_rate = 768_000.0;
    let signal_freq = 100_000.0;
    let shift_freq = -50_000.0;
    let reset_interval = 512;
    let seconds = 1;
    let num_samples = seconds * 768_000;

    let input = generate_complex_input(num_samples, sample_rate, signal_freq);

    c.bench_function("ComplexRotator 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut rot = ComplexRotator::new(shift_freq, sample_rate, reset_interval);
            for &x in &input {
                black_box(rot.rotate(black_box(x)));
            }
        });
    });
}

criterion_group!(benches, bench_rotator_768ksps);
criterion_main!(benches);
