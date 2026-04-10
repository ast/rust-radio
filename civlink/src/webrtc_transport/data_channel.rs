// Copyright SM6WJM 2026

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use sidebridge::{IcomRadio, Mode, Radio, ScopeFrame};
use tokio::sync::{Notify, oneshot};
use tokio_stream::{Stream, StreamExt};
use webrtc::peer_connection::RTCPeerConnection;

use crate::Result;
use crate::error::CivlinkError;

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    let cid = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
        if !wait_for_open(&dc).await {
            tracing::warn!(cid, "state data channel failed to open");
            return;
        }
        tracing::info!(cid, "state data channel open");

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
            if let Err(e) = dc.send_text(json).await {
                tracing::error!(cid, "initial state send failed: {e}");
            }
        }

        // Subscribe after channel open and initial state sent
        let freq_stream = subscribe_freq();
        let mode_stream = subscribe_mode();

        pump_state(cid, dc, &mut state, freq_stream, mode_stream).await;
        tracing::info!(cid, "state data channel closed");
    });

    Ok(())
}

async fn pump_state(
    cid: u64,
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
            else => {
                tracing::debug!(cid, "state pump: all streams ended");
                break;
            }
        };
        let json = match serde_json::to_string(&state) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!(cid, "failed to serialize radio state: {e}");
                continue;
            }
        };
        if let Err(e) = dc.send_text(json).await {
            tracing::debug!(cid, "state pump: send failed: {e}");
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Spectrum channel — raw scope frames
// ---------------------------------------------------------------------------

/// Create a "spectrum" data channel that streams scope frames.
///
/// The client can send any message on this channel to trigger a resubscribe
/// to the scope stream (e.g. if the waterfall stopped updating).
pub async fn setup_spectrum_channel(
    pc: &Arc<RTCPeerConnection>,
    subscribe: impl Fn() -> Box<dyn Stream<Item = ScopeFrame> + Send + Unpin> + Send + 'static,
) -> Result<()> {
    let dc = pc
        .create_data_channel("spectrum", None)
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to create spectrum channel: {e}")))?;

    let cid = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);

    // Signal from on_message handler to the pump loop
    let restart = Arc::new(Notify::new());
    let restart_clone = Arc::clone(&restart);
    dc.on_message(Box::new(move |_msg| {
        tracing::info!(cid, "spectrum restart requested by client");
        restart_clone.notify_one();
        Box::pin(async {})
    }));

    tokio::spawn(async move {
        if !wait_for_open(&dc).await {
            tracing::warn!(cid, "spectrum data channel failed to open");
            return;
        }
        tracing::info!(cid, "spectrum data channel open");

        loop {
            let mut scope = subscribe();
            let reason = pump_spectrum(cid, &dc, &mut scope, &restart).await;
            match reason {
                PumpExit::Restart => {
                    tracing::info!(cid, "spectrum pump restarting");
                    continue;
                }
                PumpExit::SendFailed | PumpExit::StreamEnded => {
                    tracing::info!(cid, "spectrum data channel closed ({reason:?})");
                    break;
                }
            }
        }
    });

    Ok(())
}

#[derive(Debug)]
enum PumpExit {
    Restart,
    SendFailed,
    StreamEnded,
}

async fn pump_spectrum(
    cid: u64,
    dc: &Arc<webrtc::data_channel::RTCDataChannel>,
    scope: &mut (dyn Stream<Item = ScopeFrame> + Send + Unpin),
    restart: &Notify,
) -> PumpExit {
    loop {
        tokio::select! {
            frame = scope.next() => {
                let Some(frame) = frame else {
                    return PumpExit::StreamEnded;
                };
                let json = match serde_json::to_string(&frame) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!(cid, "failed to serialize scope frame: {e}");
                        continue;
                    }
                };
                if let Err(e) = dc.send_text(json).await {
                    tracing::debug!(cid, "spectrum pump: send failed: {e}");
                    return PumpExit::SendFailed;
                }
            }
            _ = restart.notified() => {
                return PumpExit::Restart;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wait for the data channel to open using the `on_open` callback.
///
/// Unlike polling `ready_state()`, `on_open` handles the case where the
/// channel is already open when the callback is registered, and avoids
/// missing fast state transitions. A 15-second timeout prevents hanging
/// if the SCTP association fails to establish.
async fn wait_for_open(dc: &Arc<webrtc::data_channel::RTCDataChannel>) -> bool {
    let (tx, rx) = oneshot::channel::<()>();
    dc.on_open(Box::new(move || {
        let _ = tx.send(());
        Box::pin(async {})
    }));
    match tokio::time::timeout(Duration::from_secs(15), rx).await {
        Ok(Ok(())) => true,
        Ok(Err(_)) => {
            tracing::warn!("data channel on_open sender dropped before firing");
            false
        }
        Err(_) => {
            tracing::warn!("data channel open timed out after 15s");
            false
        }
    }
}
