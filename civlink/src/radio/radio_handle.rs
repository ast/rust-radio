// Copyright SM6WJM 2026

use std::sync::Arc;

use sidebridge::{IcomRadio, Mode, Radio, RadioEvent, RadioScope, ScopeFrame};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use url::Url;

use crate::Result;
use crate::error::CivlinkError;

/// Wraps the sidebridge IcomRadio, fans out the single event stream
/// to multiple broadcast subscribers (scope, freq, mode).
pub struct RadioHandle {
    radio: Arc<IcomRadio>,
    scope_tx: broadcast::Sender<ScopeFrame>,
    freq_tx: broadcast::Sender<u64>,
    mode_tx: broadcast::Sender<(Mode, u8)>,
}

impl RadioHandle {
    /// Connect to an Icom radio, enable scope output, and start the
    /// event fan-out task.
    pub async fn connect(url: &Url) -> Result<Self> {
        let radio = IcomRadio::connect(url).await?;
        radio
            .set_scope_output(true)
            .await
            .map_err(CivlinkError::Radio)?;
        tracing::info!("radio connected, scope output enabled");

        let event_rx = radio
            .take_event_stream()
            .expect("event stream not yet taken");

        let (scope_tx, _) = broadcast::channel(64);
        let (freq_tx, _) = broadcast::channel(16);
        let (mode_tx, _) = broadcast::channel(16);

        // Spawn fan-out task: reads from single event stream, broadcasts to subscribers
        let scope_tx_clone = scope_tx.clone();
        let freq_tx_clone = freq_tx.clone();
        let mode_tx_clone = mode_tx.clone();
        tokio::spawn(Self::fan_out_task(
            event_rx,
            scope_tx_clone,
            freq_tx_clone,
            mode_tx_clone,
        ));

        Ok(Self {
            radio: Arc::new(radio),
            scope_tx,
            freq_tx,
            mode_tx,
        })
    }

    /// Get a new scope frame stream. Each caller gets an independent subscriber.
    pub fn scope_stream(&self) -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin> {
        let rx = self.scope_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| r.ok()))
    }

    /// Get a new frequency update stream.
    pub fn freq_stream(&self) -> Box<dyn Stream<Item = u64> + Send + Unpin> {
        let rx = self.freq_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| r.ok()))
    }

    /// Get a new mode update stream.
    pub fn mode_stream(&self) -> Box<dyn Stream<Item = (Mode, u8)> + Send + Unpin> {
        let rx = self.mode_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| r.ok()))
    }

    /// Get a shared reference to the underlying radio.
    pub fn radio(&self) -> Arc<IcomRadio> {
        Arc::clone(&self.radio)
    }

    async fn fan_out_task(
        mut event_rx: tokio::sync::mpsc::Receiver<RadioEvent>,
        scope_tx: broadcast::Sender<ScopeFrame>,
        freq_tx: broadcast::Sender<u64>,
        mode_tx: broadcast::Sender<(Mode, u8)>,
    ) {
        while let Some(event) = event_rx.recv().await {
            match event {
                RadioEvent::Scope(frame) => {
                    let _ = scope_tx.send(frame);
                }
                RadioEvent::Frequency(hz) => {
                    let _ = freq_tx.send(hz);
                }
                RadioEvent::Mode(mode, filter) => {
                    let _ = mode_tx.send((mode, filter));
                }
            }
        }
        tracing::info!("radio event stream ended, fan-out task stopping");
    }
}
