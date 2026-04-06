use criterion::{Criterion, criterion_group, criterion_main};

use filters::ChainableDecimator;
use filters::Decimator;
use filters::FirDecimator;
use filters::kernels::{FM_32, FM_64, HB_35};
use filters::{Filter, FirFilter, FirFilter3, FirFilter4};
use signals::ComplexOscillator;

use num_complex::Complex32;
use std::hint::black_box;

/// Generate 1 second of IQ samples at 768 kHz — a typical AirspyHF+ rate.
fn iq_768k() -> Vec<Complex32> {
    ComplexOscillator::new(50_000.0, 768_000.0)
        .take(768_000)
        .collect()
}

/// FIR filter implementations compared on the same 768k Complex32 input.
/// This is the core inner loop of any SDR receiver — every sample passes
/// through at least one FIR before decimation.
fn bench_fir_implementations(c: &mut Criterion) {
    let input = iq_768k();
    let coeffs = HB_35;

    let mut group = c.benchmark_group("FIR 35-tap 768k Complex32");

    group.bench_function("FirFilter (naive/VecDeque)", |b| {
        b.iter(|| {
            let mut fir = FirFilter::new(coeffs.to_vec());
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    group.bench_function("FirFilter3 (RingBuffer/Vec)", |b| {
        b.iter(|| {
            let mut fir = FirFilter3::new(coeffs.to_vec());
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    group.bench_function("FirFilter4 (RingBuffer/const N)", |b| {
        b.iter(|| {
            let mut fir = FirFilter4::new(coeffs);
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    group.finish();
}

/// Decimation is where you get the real throughput win in SDR: the dot product
/// is only computed every D-th sample. Compare decimate-by-2 with different
/// filter lengths — shorter filters trade stopband rejection for speed.
fn bench_decimation(c: &mut Criterion) {
    let input = iq_768k();

    let mut group = c.benchmark_group("Decimation 768k Complex32");

    // HB_35 decimate-by-2: 768k → 384k
    group.bench_function("HB35 decimate-by-2", |b| {
        b.iter(|| {
            let mut dec: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);
            for &x in &input {
                black_box(dec.decimate(black_box(x)));
            }
        });
    });

    // FM_32 decimate-by-2: 768k → 384k (fewer taps, wider transition band)
    group.bench_function("FM32 decimate-by-2", |b| {
        b.iter(|| {
            let mut dec: FirDecimator<Complex32, 32, 2> = FirDecimator::new(FM_32);
            for &x in &input {
                black_box(dec.decimate(black_box(x)));
            }
        });
    });

    // FM_64 decimate-by-4: 768k → 192k
    group.bench_function("FM64 decimate-by-4", |b| {
        b.iter(|| {
            let mut dec: FirDecimator<Complex32, 64, 4> = FirDecimator::new(FM_64);
            for &x in &input {
                black_box(dec.decimate(black_box(x)));
            }
        });
    });

    group.finish();
}

/// Chained decimation: the practical SDR pipeline for going from 768 kHz
/// to a narrow audio bandwidth. Each stage halves or quarters the rate,
/// so later stages process far fewer samples.
fn bench_chain_decimation(c: &mut Criterion) {
    let input = iq_768k();

    let mut group = c.benchmark_group("Chain decimation 768k Complex32");

    // 768k → 192k → 48k → 24k  (÷4, ÷4, ÷2 = ÷32 total)
    group.bench_function("3-stage HB35 (÷4, ÷4, ÷2 = ÷32)", |b| {
        b.iter(|| {
            let fir0: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
            let fir1: FirDecimator<Complex32, 35, 4> = FirDecimator::new(HB_35);
            let fir2: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);

            let mut chain = fir0.chain_with(fir1).chain_with(fir2);

            for &x in &input {
                black_box(chain.decimate(black_box(x)));
            }
        });
    });

    // 768k → 384k → 192k → 96k → 48k  (four ÷2 stages = ÷16)
    group.bench_function("4-stage HB35 (÷2 × 4 = ÷16)", |b| {
        b.iter(|| {
            let fir0: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);
            let fir1: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);
            let fir2: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);
            let fir3: FirDecimator<Complex32, 35, 2> = FirDecimator::new(HB_35);

            let mut chain = fir0.chain_with(fir1).chain_with(fir2).chain_with(fir3);

            for &x in &input {
                black_box(chain.decimate(black_box(x)));
            }
        });
    });

    // FM demod path: FM_64 ÷4 then FM_32 ÷2 (768k → 192k → 96k)
    group.bench_function("FM path (FM64 ÷4 → FM32 ÷2 = ÷8)", |b| {
        b.iter(|| {
            let fir0: FirDecimator<Complex32, 64, 4> = FirDecimator::new(FM_64);
            let fir1: FirDecimator<Complex32, 32, 2> = FirDecimator::new(FM_32);

            let mut chain = fir0.chain_with(fir1);

            for &x in &input {
                black_box(chain.decimate(black_box(x)));
            }
        });
    });

    group.finish();
}

/// Real-valued f32 filter — relevant for audio-rate processing after
/// demodulation (e.g. de-emphasis, audio bandpass).
fn bench_fir_f32(c: &mut Criterion) {
    let input: Vec<f32> = ComplexOscillator::new(5_000.0, 48_000.0)
        .take(48_000)
        .map(|c| c.re)
        .collect();
    let coeffs = HB_35;

    let mut group = c.benchmark_group("FIR 35-tap 48k f32");

    group.bench_function("FirFilter (naive)", |b| {
        b.iter(|| {
            let mut fir = FirFilter::new(coeffs.to_vec());
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    group.bench_function("FirFilter4 (const N)", |b| {
        b.iter(|| {
            let mut fir = FirFilter4::new(coeffs);
            for &x in &input {
                black_box(fir.filter(black_box(x)));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fir_implementations,
    bench_decimation,
    bench_chain_decimation,
    bench_fir_f32,
);
criterion_main!(benches);
