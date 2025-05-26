use criterion::{criterion_group, criterion_main, Criterion};

use num_complex::Complex32;

use signals::ComplexOscillator;

// Black box is used to prevent the compiler from optimizing away the
// filter calls
use std::hint::black_box;

use spectrum::Analyzer;

fn bench_analyzer_768ksps(c: &mut Criterion) {
    let num_samples = 2 * 32768;

    // Create a complex oscillator with a frequency of 50 kHz and sample rate of 768 kHz
    let sample_rate = 768_000.0;
    let frequency = 256_000.0;
    let osc = ComplexOscillator::new(frequency, sample_rate);

    let input = osc.take(num_samples).collect::<Vec<Complex32>>();
    let mut output = vec![f32::default(); num_samples];

    let mut analyzer = Analyzer::new(num_samples, sample_rate);

    c.bench_function("Analyzer", |b| {
        b.iter(|| {
            analyzer.process(black_box(&input), black_box(&mut output));
            black_box(());
        });
    });
}

criterion_group!(benches, bench_analyzer_768ksps);
criterion_main!(benches);
