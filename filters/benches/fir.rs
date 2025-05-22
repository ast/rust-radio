use criterion::{criterion_group, criterion_main, Criterion};
use filters::{Filter, FirFilter, FirFilter3, FirFilter4};

use filters::StackFirFilter;

use filters::kernels::HB_35;

use num_complex::Complex32;

// Black box is used to prevent the compiler from optimizing away the
// filter calls
use std::hint::black_box;

fn generate_input(len: usize) -> Vec<f32> {
    (0..len).map(|i| (i as f32).sin()).collect()
}

fn generate_complex_input(len: usize) -> Vec<Complex32> {
    (0..len)
        .map(|i| {
            let t = i as f32;

            let (im, re) = t.sin_cos();
            Complex32::new(re, im)
        })
        .collect()
}

fn bench_fir_768ksps(c: &mut Criterion) {
    let num_samples = 768_000;

    // f32 input
    let input = generate_input(num_samples);

    // Complex32 input
    let complex_input = generate_complex_input(num_samples);

    let coeffs = HB_35;

    c.bench_function("FirFilter2 (optimized) 768k f32 samples", |b| {
        b.iter(|| {
            let mut fir = FirFilter3::new(coeffs.to_vec());
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    c.bench_function("FirFilter2 (optimized) 768k Complex32 samples", |b| {
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

    // StackFirFilter
    c.bench_function("StackFirFilter 768k Complex32 samples", |b| {
        b.iter(|| {
            let mut fir = StackFirFilter::<Complex32, 35, 64>::new(coeffs);
            for &x in &complex_input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });
}

criterion_group!(benches, bench_fir_768ksps);
criterion_main!(benches);
