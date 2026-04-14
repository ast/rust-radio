use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use bytes::Bytes;
use doublemap::Consumer;
use filters::rotate::ComplexRotator;
use num_complex::Complex32;
use opus::{Application, Channels, Encoder};
use tokio::sync::{broadcast, watch};
use tracing::{debug, error, warn};

use super::demod::{AUDIO_RATE, Demod, IQ_RATE, Mode, build};

/// Opus frame: 20 ms @ 48 kHz mono = 960 samples.
const OPUS_FRAME_SAMPLES: usize = (AUDIO_RATE as usize / 1000) * 20;

/// An Opus-encoded audio frame ready for a WebRTC track pump.
#[derive(Clone, Debug)]
pub struct AudioFrame {
    pub data: Bytes,
    pub duration_ms: u32,
}

/// Per-session demod handle. Drop to terminate the pipeline thread.
pub struct DemodHandle {
    audio_tx: broadcast::Sender<AudioFrame>,
    offset_tx: watch::Sender<f32>,
    mode_tx: watch::Sender<Mode>,
    filter_rx: watch::Receiver<(f32, f32)>,
    shutdown: Arc<AtomicBool>,
}

impl DemodHandle {
    pub fn subscribe_audio(&self) -> broadcast::Receiver<AudioFrame> {
        self.audio_tx.subscribe()
    }

    pub fn set_offset_hz(&self, hz: f32) {
        let _ = self.offset_tx.send(hz);
    }

    pub fn offset_hz(&self) -> f32 {
        *self.offset_tx.borrow()
    }

    pub fn set_mode(&self, mode: Mode) {
        let _ = self.mode_tx.send(mode);
    }

    pub fn mode(&self) -> Mode {
        *self.mode_tx.borrow()
    }

    /// Current demod passband `(low_hz, high_hz)` relative to the demod
    /// center. Updated by the worker whenever the demod is rebuilt.
    pub fn filter_hz(&self) -> (f32, f32) {
        *self.filter_rx.borrow()
    }
}

impl Drop for DemodHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Spawn the demod + Opus encode thread. The pipeline checks `offset` and
/// `mode` watch channels between IQ blocks; offset changes are applied
/// click-free by `ComplexRotator::set_freq`, mode changes rebuild the
/// demod chain (brief audio transient).
pub fn spawn(
    consumer: Consumer<Complex32>,
    initial_offset_hz: f32,
    initial_mode: Mode,
) -> DemodHandle {
    let (audio_tx, _) = broadcast::channel::<AudioFrame>(25);
    let (offset_tx, mut offset_rx) = watch::channel(initial_offset_hz);
    let (mode_tx, mut mode_rx) = watch::channel(initial_mode);
    let initial = build(initial_mode);
    let (filter_tx, filter_rx) = watch::channel((initial.filter_low_hz(), initial.filter_high_hz()));
    let shutdown = Arc::new(AtomicBool::new(false));

    let tx = audio_tx.clone();
    let shutdown_flag = shutdown.clone();

    thread::Builder::new()
        .name("sdrlink-demod".to_string())
        .spawn(move || {
            let mut current_offset = initial_offset_hz;
            let mut rotator = ComplexRotator::new(-current_offset, IQ_RATE as f32, 512);
            let mut demod: Box<dyn Demod> = initial;

            let mut encoder = match Encoder::new(AUDIO_RATE, Channels::Mono, Application::Audio) {
                Ok(e) => e,
                Err(e) => {
                    error!("failed to create opus encoder: {e}");
                    return;
                }
            };

            let mut frame_buf: Vec<f32> = Vec::with_capacity(OPUS_FRAME_SAMPLES);
            let mut out_buf = vec![0u8; 4000];

            loop {
                if shutdown_flag.load(Ordering::Relaxed) {
                    break;
                }
                if offset_rx.has_changed().unwrap_or(false) {
                    current_offset = *offset_rx.borrow_and_update();
                    rotator.set_freq(-current_offset, IQ_RATE as f32);
                }
                if mode_rx.has_changed().unwrap_or(false) {
                    let m = *mode_rx.borrow_and_update();
                    demod = build(m);
                    let _ = filter_tx.send((demod.filter_low_hz(), demod.filter_high_hz()));
                    frame_buf.clear();
                }

                let res = consumer.consume(|input| {
                    for &iq in input {
                        let shifted = rotator.rotate(iq);
                        let Some(a) = demod.process(shifted) else { continue };
                        frame_buf.push(a);

                        if frame_buf.len() == OPUS_FRAME_SAMPLES {
                            if tx.receiver_count() > 0 {
                                match encoder.encode_float(&frame_buf, &mut out_buf) {
                                    Ok(n) => {
                                        let frame = AudioFrame {
                                            data: Bytes::copy_from_slice(&out_buf[..n]),
                                            duration_ms: 20,
                                        };
                                        let _ = tx.send(frame);
                                    }
                                    Err(e) => warn!("opus encode error: {e}"),
                                }
                            }
                            frame_buf.clear();
                        }
                    }
                    input.len()
                });

                if res.is_err() {
                    debug!("demod consumer disconnected, stopping worker");
                    break;
                }
            }
        })
        .expect("failed to spawn demod pipeline thread");

    DemodHandle {
        audio_tx,
        offset_tx,
        mode_tx,
        filter_rx,
        shutdown,
    }
}
