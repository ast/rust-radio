use std::sync::Arc;
use std::thread;

use doublemap::Consumer;
use num_complex::Complex32;
use pool::BufferPool;
use spectrum::Analyzer;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Quantization range for spectrum power (dBFS).
const DB_MIN: f32 = -140.0;
const DB_MAX: f32 = -20.0;

/// A broadcast frame: one fully-quantized PDS, `fft_len` bytes.
///
/// Clients perform per-viewport min/max decimation on this in a later stage.
pub type SpectrumFrame = Arc<[u8]>;

/// Spawns a worker thread: Consumer<Complex32> -> Analyzer -> quantize to u8 ->
/// broadcast. Returns the broadcast sender; any number of clients can
/// `subscribe()`.
///
/// `decimate_by` drops every N-1 of N frames to reach `fft_rate_hz`: the
/// caller is expected to compute it from samplerate / fft_len / fft_rate.
pub fn spawn(
    consumer: Consumer<Complex32>,
    fft_len: usize,
    samplerate: u32,
    decimate_by: u32,
) -> broadcast::Sender<SpectrumFrame> {
    let (tx, _) = broadcast::channel::<SpectrumFrame>(8);
    let sender = tx.clone();

    thread::Builder::new()
        .name("sdrlink-spectrum".to_string())
        .spawn(move || {
            let mut analyzer = Analyzer::new(fft_len, samplerate as f32);
            let pds_pool = BufferPool::<f32>::new(4, fft_len);
            let mut frame_counter: u32 = 0;

            loop {
                let result = consumer.consume(|input| {
                    // Only produce a frame every `decimate_by` blocks.
                    frame_counter = frame_counter.wrapping_add(1);
                    let emit = decimate_by == 0 || frame_counter % decimate_by == 0;

                    if emit && tx.receiver_count() > 0 {
                        let mut pds = pds_pool.get();
                        analyzer.process(&input[..fft_len], &mut pds);

                        // Quantize f32 dBFS -> u8 over [DB_MIN, DB_MAX].
                        let scale = 255.0 / (DB_MAX - DB_MIN);
                        let mut out = vec![0u8; fft_len];
                        for (dst, &db) in out.iter_mut().zip(pds.iter()) {
                            let clamped = db.clamp(DB_MIN, DB_MAX);
                            *dst = ((clamped - DB_MIN) * scale).round() as u8;
                        }
                        let frame: SpectrumFrame = Arc::from(out.into_boxed_slice());
                        if let Err(e) = tx.send(frame) {
                            debug!("no spectrum receivers: {e}");
                        }
                    }

                    fft_len
                });

                if result.is_err() {
                    warn!("spectrum consumer disconnected, stopping worker");
                    break;
                }
            }
        })
        .expect("failed to spawn spectrum worker thread");

    sender
}
