use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use bytes::Bytes;
use doublemap::Consumer;
use filters::{
    Decimator, Deemphasis, FirDecimator, FmDemod,
    kernels::{FM_64, HB_35},
    rotate::ComplexRotator,
};
use num_complex::Complex32;
use opus::{Application, Channels, Encoder};
use tokio::sync::{broadcast, watch};
use tracing::{debug, error, warn};

/// Capture rate fed into the pipeline.
const IQ_RATE: u32 = 768_000;
/// Post-stage-1 rate (÷4).
const DEMOD_RATE: u32 = IQ_RATE / 4;
/// Audio rate (÷2 ÷2 after demod).
pub const AUDIO_RATE: u32 = DEMOD_RATE / 4;
/// Opus frame: 20 ms @ 48 kHz mono = 960 samples.
const OPUS_FRAME_SAMPLES: usize = (AUDIO_RATE as usize / 1000) * 20;
/// Broadcast-FM peak deviation.
const FM_DEVIATION_HZ: f32 = 75_000.0;
/// FM de-emphasis time constant: 50 µs (ITU R1, Europe).
const FM_DEEMPHASIS_TAU_S: f32 = 50e-6;

/// FM demod's effective filter passband, relative to the demod center.
/// FM_64 is a wideband LP that passes the full FM channel.
pub const FM_FILTER_LOW_HZ: f32 = -100_000.0;
pub const FM_FILTER_HIGH_HZ: f32 = 100_000.0;

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
}

impl Drop for DemodHandle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Spawn the FM demod + Opus encode thread. The pipeline checks the
/// `offset_hz` watch between IQ blocks and retunes the rotator without
/// disturbing phase (click-free).
pub fn spawn(consumer: Consumer<Complex32>, initial_offset_hz: f32) -> DemodHandle {
    let (audio_tx, _) = broadcast::channel::<AudioFrame>(25);
    let (offset_tx, mut offset_rx) = watch::channel(initial_offset_hz);
    let shutdown = Arc::new(AtomicBool::new(false));

    let tx = audio_tx.clone();
    let shutdown_flag = shutdown.clone();

    thread::Builder::new()
        .name("sdrlink-fm".to_string())
        .spawn(move || {
            let mut current_offset = initial_offset_hz;
            let mut rotator = ComplexRotator::new(-current_offset, IQ_RATE as f32, 512);
            let mut stage1: FirDecimator<Complex32, 64, 4> = FirDecimator::new(FM_64);
            let mut demod = FmDemod::new(DEMOD_RATE as f32, FM_DEVIATION_HZ);
            let mut stage2: FirDecimator<f32, 35, 2> = FirDecimator::new(HB_35);
            let mut stage3: FirDecimator<f32, 35, 2> = FirDecimator::new(HB_35);
            let mut deemph = Deemphasis::new(AUDIO_RATE as f32, FM_DEEMPHASIS_TAU_S);

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

                let res = consumer.consume(|input| {
                    for &iq in input {
                        let shifted = rotator.rotate(iq);
                        let Some(iq_dec) = stage1.decimate(shifted) else { continue };
                        let audio = demod.demod(iq_dec);
                        let Some(a1) = stage2.decimate(audio) else { continue };
                        let Some(a2) = stage3.decimate(a1) else { continue };
                        let a2 = deemph.process(a2);
                        frame_buf.push(a2);

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
                    debug!("fm consumer disconnected, stopping worker");
                    break;
                }
            }
        })
        .expect("failed to spawn fm pipeline thread");

    DemodHandle { audio_tx, offset_tx, shutdown }
}
