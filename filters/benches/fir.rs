use criterion::{Criterion, criterion_group, criterion_main};

use filters::ChainableDecimator;
use filters::Decimator;
use filters::FirDecimator;
use filters::kernels::HB_35;
use filters::{Filter, FirFilter3, FirFilter4};
use signals::ComplexOscillator;

use num_complex::Complex32;

// Black box is used to prevent the compiler from optimizing away the
// filter calls
use std::hint::black_box;

fn bench_fir_768ksps(c: &mut Criterion) {
    let num_samples = 768_000;

    let osc = ComplexOscillator::new(50_000.0, 768_000.0);

    let complex_input = osc.take(num_samples).collect::<Vec<_>>();
    let input = complex_input.iter().map(|c| c.re).collect::<Vec<f32>>();

    let coeffs = HB_35;

    c.bench_function("FirFilter3 (optimized) 768k f32 samples", |b| {
        b.iter(|| {
            let mut fir = FirFilter3::new(coeffs.to_vec());
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    c.bench_function("FirFilter3 (optimized) 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut fir = FirFilter3::new(coeffs.to_vec());
            for &x in &complex_input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    // FirFilter4 (coeffs on stack)
    c.bench_function("FirFilter4 768k Complex32 samples", |b| {
        b.iter(|| {
            // Convert taps to array

            let mut fir = FirFilter4::new(coeffs);
            for &x in &complex_input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    // FirDecimator
    c.bench_function("FirDecimator 768k Complex32 samples", |b| {
        b.iter(|| {
            // Convert taps to array

            let mut fir: FirDecimator<Complex32, 35, 2> = FirDecimator::new(coeffs);
            //let mut fir = FirDecimator::new(coeffs);
            for &x in &complex_input {
                black_box(fir.decimate(black_box(x)));
            }
        });
    });

    // FirDecimator in 3 steps
    c.bench_function("FirDecimator in 3 steps 768k Complex32 samples", |b| {
        b.iter(|| {
            let fir0: FirDecimator<Complex32, 35, 4> = FirDecimator::new(coeffs);
            let fir1: FirDecimator<Complex32, 35, 4> = FirDecimator::new(coeffs);
            let fir2: FirDecimator<Complex32, 35, 2> = FirDecimator::new(coeffs);

            let mut chain = fir0.chain_with(fir1).chain_with(fir2);

            for &x in &complex_input {
                black_box(chain.decimate(black_box(x)));
            }
        });
    });
}

criterion_group!(benches, bench_fir_768ksps);
criterion_main!(benches);
