use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use num_complex::Complex32;
use signals::complex_oscillator::ComplexOscillator;
use tracing::info;

use super::IqBroker;

/// Block size (samples per broadcast) — matches AirspyHF's native output size.
pub const BLOCK_SIZE: usize = 4096;

/// Spawns a thread that pushes a pure tone into the broker at `samplerate`.
///
/// Used on dev machines without an AirspyHF attached. The generated signal
/// is `e^(j*2π*tone_hz*t)`, so a single spectral line appears at
/// `center + tone_hz` (here: at bin `fft_len/2 + bin_offset`).
pub fn spawn(
    broker: Arc<IqBroker>,
    samplerate: u32,
    tone_hz: f32,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("sdrlink-fake-source".to_string())
        .spawn(move || {
            info!(samplerate, tone_hz, "fake IQ source running");

            let mut osc = ComplexOscillator::new(tone_hz, samplerate as f32);
            let mut block = vec![Complex32::default(); BLOCK_SIZE];

            // Pace at real-time: one block every BLOCK_SIZE / samplerate seconds.
            let period = Duration::from_secs_f64(BLOCK_SIZE as f64 / samplerate as f64);
            let mut next = Instant::now();

            while !stop.load(Ordering::Relaxed) {
                for s in block.iter_mut() {
                    *s = osc.next().unwrap();
                }
                broker.broadcast(&block);

                next += period;
                if let Some(sleep) = next.checked_duration_since(Instant::now()) {
                    thread::sleep(sleep);
                } else {
                    // Fell behind — resync so we don't burst-catch-up forever.
                    next = Instant::now();
                }
            }

            info!("fake IQ source stopped");
        })
        .expect("failed to spawn fake source thread")
}
