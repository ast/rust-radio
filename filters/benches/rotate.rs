use criterion::{Criterion, criterion_group, criterion_main};

use filters::rotate::ComplexRotator;
use signals::ComplexOscillator;

use num_complex::Complex32;
use std::hint::black_box;

/// Frequency shifting via complex rotation — used for tuning within the
/// digitized bandwidth. This is applied to every sample before filtering,
/// so it must be faster than the FIR path.
fn bench_rotator(c: &mut Criterion) {
    let sample_rate = 768_000.0;
    let num_samples = 768_000;

    let osc = ComplexOscillator::new(100_000.0, sample_rate);
    let input: Vec<Complex32> = osc.take(num_samples).collect();

    let mut group = c.benchmark_group("ComplexRotator 768k");

    group.bench_function("rotate -50 kHz", |b| {
        b.iter(|| {
            let mut rot = ComplexRotator::new(-50_000.0, sample_rate, 512);
            for &x in &input {
                black_box(rot.rotate(black_box(x)));
            }
        });
    });

    // Measure with a longer renormalization interval — less division overhead
    group.bench_function("rotate -50 kHz (renorm every 4096)", |b| {
        b.iter(|| {
            let mut rot = ComplexRotator::new(-50_000.0, sample_rate, 4096);
            for &x in &input {
                black_box(rot.rotate(black_box(x)));
            }
        });
    });

    group.finish();
}

/// Combined rotate + decimate — the typical first stage of an SDR receiver.
/// Tune to a signal within the digitized bandwidth, then decimate.
fn bench_rotate_and_decimate(c: &mut Criterion) {
    use filters::ChainableDecimator;
    use filters::Decimator;
    use filters::FirDecimator;
    use filters::kernels::HB_35;

    let sample_rate = 768_000.0;
    let num_samples = 768_000;

    let osc = ComplexOscillator::new(100_000.0, sample_rate);
    let input: Vec<Complex32> = osc.take(num_samples).collect();

    c.bench_function("rotate + 3-stage decimate ÷32 768k", |b| {
        b.iter(|| {
            let mut rot = ComplexRotator::new(-100_000.0, sample_rate, 512);

            let fir0: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
            let fir1: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
            let fir2: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);
            let mut chain = fir0.chain_with(fir1).chain_with(fir2);

            for &x in &input {
                let shifted = rot.rotate(x);
                black_box(chain.decimate(black_box(shifted)));
            }
        });
    });
}

criterion_group!(benches, bench_rotator, bench_rotate_and_decimate);
criterion_main!(benches);
