// Copyright SM6WJM 2026

use std::sync::Arc;

use sidebridge::{IcomRadio, Mode, RadioScope, ScopeFrame};
use tokio_stream::Stream;
use url::Url;

use crate::Result;
use crate::error::CivlinkError;

/// Wraps the sidebridge IcomRadio and manages scope output.
pub struct RadioHandle {
    radio: Arc<IcomRadio>,
}

impl RadioHandle {
    /// Connect to an Icom radio and enable scope waveform output.
    pub async fn connect(url: &Url) -> Result<Self> {
        let radio = IcomRadio::connect(url).await?;
        radio
            .set_scope_output(true)
            .await
            .map_err(|e| CivlinkError::Radio(e))?;
        tracing::info!("radio connected, scope output enabled");
        Ok(Self {
            radio: Arc::new(radio),
        })
    }

    /// Get a new scope frame stream. Each caller gets an independent subscriber.
    pub fn scope_stream(&self) -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin> {
        self.radio.scope_stream()
    }

    /// Get a new frequency update stream. Receives both solicited and
    /// unsolicited frequency changes from the radio.
    pub fn freq_stream(&self) -> Box<dyn Stream<Item = u64> + Send + Unpin> {
        self.radio.freq_stream()
    }

    /// Get a new mode update stream. Receives both solicited and
    /// unsolicited mode changes from the radio.
    pub fn mode_stream(&self) -> Box<dyn Stream<Item = (Mode, u8)> + Send + Unpin> {
        self.radio.mode_stream()
    }

    /// Get a shared reference to the underlying radio.
    pub fn radio(&self) -> Arc<IcomRadio> {
        Arc::clone(&self.radio)
    }
}
