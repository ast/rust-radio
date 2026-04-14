use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;

use airspyhf::Device;
use tokio::sync::broadcast;

use crate::config::SdrConfig;
use crate::error::{Result, SdrLinkError};

use super::fm_pipeline::AudioFrame;
use super::spectrum_worker::SpectrumFrame;
use super::{IqBroker, airspyhf_source, fake_source, fm_pipeline, spectrum_worker};

/// AirspyHF+ delivers 4096 Complex32 samples per callback invocation.
const AIRSPYHF_BLOCK_SIZE: usize = 4096;

/// Owns the IQ source, the [`IqBroker`], and the spawned workers.
pub struct SdrHandle {
    broker: Arc<IqBroker>,
    spectrum_tx: broadcast::Sender<SpectrumFrame>,
    audio_tx: broadcast::Sender<AudioFrame>,
    center_tx: broadcast::Sender<f64>,
    center_hz: RwLock<f64>,
    source: IqSource,
    config: SdrConfig,
}

/// Two variants so dev machines (and integration tests) can run without an
/// AirspyHF attached.
enum IqSource {
    Airspyhf(Option<Device>),
    Fake {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    },
}

impl SdrHandle {
    /// Open an AirspyHF+, tune + start streaming, and spin the spectrum worker.
    pub fn start_hardware(config: SdrConfig) -> Result<Self> {
        let broker = Arc::new(IqBroker::new(AIRSPYHF_BLOCK_SIZE));
        let spectrum_tx = Self::spawn_spectrum(&broker, &config);
        let audio_tx = Self::spawn_fm(&broker);

        let device = airspyhf_source::start(broker.clone(), config.center_hz, config.samplerate)?;
        let (center_tx, _) = broadcast::channel::<f64>(8);

        Ok(Self {
            broker,
            spectrum_tx,
            audio_tx,
            center_tx,
            center_hz: RwLock::new(config.center_hz),
            source: IqSource::Airspyhf(Some(device)),
            config,
        })
    }

    /// Dev-machine mode: fake oscillator producing a tone at `tone_hz` relative
    /// to the configured center frequency.
    pub fn start_fake(config: SdrConfig, tone_hz: f32) -> Self {
        let broker = Arc::new(IqBroker::new(fake_source::BLOCK_SIZE));
        let spectrum_tx = Self::spawn_spectrum(&broker, &config);
        let audio_tx = Self::spawn_fm(&broker);

        let stop = Arc::new(AtomicBool::new(false));
        let thread = fake_source::spawn(broker.clone(), config.samplerate, tone_hz, stop.clone());
        let (center_tx, _) = broadcast::channel::<f64>(8);

        Self {
            broker,
            spectrum_tx,
            audio_tx,
            center_tx,
            center_hz: RwLock::new(config.center_hz),
            source: IqSource::Fake {
                stop,
                thread: Some(thread),
            },
            config,
        }
    }

    fn spawn_fm(broker: &Arc<IqBroker>) -> broadcast::Sender<AudioFrame> {
        // Feed the pipeline block-at-a-time so the FIR ÷4 stays aligned.
        let consumer = broker.subscribe(4096);
        fm_pipeline::spawn(consumer, 0.0)
    }

    fn spawn_spectrum(
        broker: &Arc<IqBroker>,
        config: &SdrConfig,
    ) -> broadcast::Sender<SpectrumFrame> {
        let fft_len = config.fft_len as usize;
        let consumer = broker.subscribe(fft_len);

        // Analyzer ticks once per fft_len samples; drop N-1 of N ticks to
        // reach the target frame rate.
        let ticks_per_sec = config.samplerate as f32 / fft_len as f32;
        let decimate_by = (ticks_per_sec / config.fft_rate_hz as f32).max(1.0).round() as u32;

        spectrum_worker::spawn(consumer, fft_len, config.samplerate, decimate_by)
    }

    pub fn subscribe_spectrum(&self) -> broadcast::Receiver<SpectrumFrame> {
        self.spectrum_tx.subscribe()
    }

    pub fn subscribe_audio(&self) -> broadcast::Receiver<AudioFrame> {
        self.audio_tx.subscribe()
    }

    pub fn config(&self) -> &SdrConfig {
        &self.config
    }

    pub fn broker(&self) -> &Arc<IqBroker> {
        &self.broker
    }

    pub fn center_hz(&self) -> f64 {
        *self.center_hz.read().unwrap()
    }

    pub fn subscribe_center(&self) -> broadcast::Receiver<f64> {
        self.center_tx.subscribe()
    }

    /// Retune the radio. Broadcasts the new center to all WS clients.
    pub fn set_center_hz(&self, hz: f64) -> Result<()> {
        match &self.source {
            IqSource::Airspyhf(Some(device)) => {
                device.set_frequency(hz).map_err(|e| {
                    SdrLinkError::Sdr(format!("set_frequency({hz}) failed: {e}"))
                })?;
            }
            IqSource::Airspyhf(None) => {
                return Err(SdrLinkError::Sdr("device already dropped".into()));
            }
            IqSource::Fake { .. } => {
                // Fake source has no real tuner — accept the value so the UI
                // still works in dev mode.
            }
        }
        *self.center_hz.write().unwrap() = hz;
        let _ = self.center_tx.send(hz);
        Ok(())
    }
}

impl Drop for SdrHandle {
    fn drop(&mut self) {
        match &mut self.source {
            IqSource::Airspyhf(device) => {
                // Explicit stop so USB worker threads are joined before the
                // broker is dropped. Ignore errors — we're tearing down.
                if let Some(mut d) = device.take() {
                    let _ = d.stop();
                }
            }
            IqSource::Fake { stop, thread } => {
                stop.store(true, Ordering::Relaxed);
                if let Some(h) = thread.take() {
                    let _ = h.join();
                }
            }
        }
        // Spectrum worker terminates on its own when its Consumer observes
        // Disconnected after the broker is dropped.
    }
}
