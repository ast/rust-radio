// Copyright SM6WJM 2026

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use sidebridge::{IcomRadio, Mode, Radio, ScopeFrame};
use tokio_stream::{Stream, StreamExt};
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::peer_connection::RTCPeerConnection;

use crate::Result;
use crate::error::CivlinkError;

// ---------------------------------------------------------------------------
// State channel — frequency, mode, filter as a maintained struct
// ---------------------------------------------------------------------------

/// Current radio state, serialized in full on every change.
#[derive(Clone, Serialize)]
struct RadioState {
    frequency: u64,
    mode: String,
    filter: u8,
}

/// Create a "state" data channel that maintains the last-known radio state
/// and sends the full struct whenever any field changes.
pub async fn setup_state_channel(
    pc: &Arc<RTCPeerConnection>,
    radio: Arc<IcomRadio>,
    subscribe_freq: impl FnOnce() -> Box<dyn Stream<Item = u64> + Send + Unpin> + Send + 'static,
    subscribe_mode: impl FnOnce() -> Box<dyn Stream<Item = (Mode, u8)> + Send + Unpin> + Send + 'static,
) -> Result<()> {
    let dc = pc
        .create_data_channel("state", None)
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to create state channel: {e}")))?;

    tokio::spawn(async move {
        if !wait_for_open(&dc).await {
            return;
        }
        tracing::info!("state data channel open");

        // Read initial state from the radio
        let freq = radio.frequency().await.unwrap_or(0);
        let (mode, filter) = match radio.mode().await {
            Ok(m) => (m.to_string(), 1),
            Err(_) => ("USB".to_string(), 1),
        };

        let mut state = RadioState {
            frequency: freq,
            mode,
            filter,
        };

        // Send initial state
        if let Ok(json) = serde_json::to_string(&state) {
            let _ = dc.send_text(json).await;
        }

        // Subscribe after channel open and initial state sent
        let freq_stream = subscribe_freq();
        let mode_stream = subscribe_mode();

        pump_state(dc, &mut state, freq_stream, mode_stream).await;
        tracing::info!("state data channel closed");
    });

    Ok(())
}

async fn pump_state(
    dc: Arc<webrtc::data_channel::RTCDataChannel>,
    state: &mut RadioState,
    mut freq: Box<dyn Stream<Item = u64> + Send + Unpin>,
    mut mode: Box<dyn Stream<Item = (Mode, u8)> + Send + Unpin>,
) {
    loop {
        tokio::select! {
            Some(hz) = freq.next() => {
                state.frequency = hz;
            }
            Some((m, f)) = mode.next() => {
                state.mode = m.to_string();
                state.filter = f;
            }
            else => break,
        };
        let json = match serde_json::to_string(&state) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("failed to serialize radio state: {e}");
                continue;
            }
        };
        if dc.send_text(json).await.is_err() {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Spectrum channel — raw scope frames
// ---------------------------------------------------------------------------

/// Create a "spectrum" data channel that streams scope frames.
pub async fn setup_spectrum_channel(
    pc: &Arc<RTCPeerConnection>,
    subscribe: impl FnOnce() -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin> + Send + 'static,
) -> Result<()> {
    let dc = pc
        .create_data_channel("spectrum", None)
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to create spectrum channel: {e}")))?;

    tokio::spawn(async move {
        if !wait_for_open(&dc).await {
            return;
        }
        tracing::info!("spectrum data channel open");

        let scope_stream = subscribe();
        pump_spectrum(dc, scope_stream).await;
        tracing::info!("spectrum data channel closed");
    });

    Ok(())
}

async fn pump_spectrum(
    dc: Arc<webrtc::data_channel::RTCDataChannel>,
    mut scope: Box<dyn Stream<Item = ScopeFrame> + Send + Unpin>,
) {
    while let Some(frame) = scope.next().await {
        let json = match serde_json::to_string(&frame) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("failed to serialize scope frame: {e}");
                continue;
            }
        };
        if dc.send_text(json).await.is_err() {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Poll the data channel until it opens (or closes/fails).
async fn wait_for_open(dc: &Arc<webrtc::data_channel::RTCDataChannel>) -> bool {
    loop {
        match dc.ready_state() {
            RTCDataChannelState::Open => return true,
            RTCDataChannelState::Closing | RTCDataChannelState::Closed => return false,
            _ => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}
