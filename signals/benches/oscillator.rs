use criterion::{criterion_group, criterion_main, Criterion};

use signals::ComplexFs4Oscillator;
use signals::ComplexOscillator;
use signals::ComplexOscillatorFast;

use std::hint::black_box;

fn bench_oscillator_768ksps(c: &mut Criterion) {
    let frequency = 50_000.0;
    let sample_rate = 768_000.0;
    let reset_interval = 512;
    let seconds = 1;
    let num_samples = seconds * 768_000;

    c.bench_function("ComplexOscillator 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut osc = ComplexOscillatorFast::new(frequency, sample_rate, reset_interval);
            for _ in 0..num_samples {
                black_box(osc.next());
            }
        });
    });

    c.bench_function("ComplexOscillatorExact 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut osc = ComplexOscillator::new(frequency, sample_rate);
            for _ in 0..num_samples {
                black_box(osc.next());
            }
        });
    });

    c.bench_function("ComplexFs4Oscillator 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut osc = ComplexFs4Oscillator::new(1.0);
            for _ in 0..num_samples {
                black_box(osc.next());
            }
        });
    });
}

criterion_group!(benches, bench_oscillator_768ksps);
criterion_main!(benches);
