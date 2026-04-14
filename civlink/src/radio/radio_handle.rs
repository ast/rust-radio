// Copyright SM6WJM 2026

use std::sync::Arc;

use sidebridge::{IcomRadio, Mode, Radio, RadioEvent, RadioGain, RadioScope, ScopeFrame};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use url::Url;

use crate::Result;
use crate::error::CivlinkError;

/// Wraps the sidebridge IcomRadio, fans out the single event stream
/// to multiple broadcast subscribers.
pub struct RadioHandle {
    radio: Arc<IcomRadio>,
    event_tx: broadcast::Sender<RadioEvent>,
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

        let (event_tx, _) = broadcast::channel(64);

        // Spawn fan-out task: reads from single-consumer mpsc, broadcasts to all subscribers
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let mut rx = event_rx;
            while let Some(event) = rx.recv().await {
                let _ = tx.send(event);
            }
            tracing::info!("radio event stream ended, fan-out task stopping");
        });

        Ok(Self {
            radio: Arc::new(radio),
            event_tx,
        })
    }

    /// Get a new stream of all radio events. Each caller gets an independent subscriber.
    pub fn event_stream(&self) -> Box<dyn Stream<Item = RadioEvent> + Send + Unpin> {
        let rx = self.event_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| match r {
            Ok(event) => Some(event),
            Err(e) => {
                tracing::warn!("broadcast subscriber lagged: {e}");
                None
            }
        }))
    }

    /// Get a new scope frame stream.
    pub fn scope_stream(&self) -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin> {
        let rx = self.event_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| match r {
            Ok(RadioEvent::Scope(frame)) => Some(frame),
            _ => None,
        }))
    }

    /// Get a new frequency update stream.
    pub fn freq_stream(&self) -> Box<dyn Stream<Item = u64> + Send + Unpin> {
        let rx = self.event_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| match r {
            Ok(RadioEvent::Frequency(hz)) => Some(hz),
            _ => None,
        }))
    }

    /// Get a new mode update stream.
    pub fn mode_stream(&self) -> Box<dyn Stream<Item = (Mode, u8)> + Send + Unpin> {
        let rx = self.event_tx.subscribe();
        Box::new(BroadcastStream::new(rx).filter_map(|r| match r {
            Ok(RadioEvent::Mode(m, f)) => Some((m, f)),
            _ => None,
        }))
    }

    /// Trigger frequency and mode reads so current state is broadcast to all subscribers.
    pub async fn read_initial_state(&self) -> Result<()> {
        self.radio
            .read_frequency()
            .await
            .map_err(CivlinkError::Radio)?;
        self.radio.read_mode().await.map_err(CivlinkError::Radio)?;
        self.radio.read_rf_gain().await.map_err(CivlinkError::Radio)?;
        Ok(())
    }

    /// Get a shared reference to the underlying radio.
    pub fn radio(&self) -> Arc<IcomRadio> {
        Arc::clone(&self.radio)
    }
}
