use criterion::{Criterion, criterion_group, criterion_main};

use filters::rotate::ComplexRotator;
use signals::ComplexOscillator;

use std::hint::black_box;

fn bench_rotator_768ksps(c: &mut Criterion) {
    let sample_rate = 768_000.0;
    let signal_freq = 100_000.0;
    let shift_freq = -50_000.0;
    let reset_interval = 512;
    let seconds = 1;
    let num_samples = seconds * 768_000;

    let osc = ComplexOscillator::new(signal_freq, sample_rate);
    let input = osc.take(num_samples).collect::<Vec<_>>();

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
