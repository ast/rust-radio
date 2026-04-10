// Copyright SM6WJM 2026

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sidebridge::RadioEvent;
use tokio::sync::oneshot;
use tokio_stream::{Stream, StreamExt};
use webrtc::data_channel::data_channel_state::RTCDataChannelState;
use webrtc::peer_connection::RTCPeerConnection;

use crate::Result;
use crate::error::CivlinkError;

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a "radio" data channel that streams all radio events as JSON.
///
/// The event stream must be subscribed **before** calling this function so
/// that events triggered between subscription and channel-open are buffered.
pub async fn setup_radio_channel(
    pc: &Arc<RTCPeerConnection>,
    events: Box<dyn Stream<Item = RadioEvent> + Send + Unpin>,
) -> Result<()> {
    let dc = pc
        .create_data_channel("radio", None)
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to create radio channel: {e}")))?;

    let cid = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);

    tokio::spawn(async move {
        if !wait_for_open(&dc).await {
            tracing::warn!(cid, "radio data channel failed to open");
            return;
        }
        tracing::info!(cid, "radio data channel open");

        // Wait for the SCTP transport to be fully ready
        wait_for_ready(&dc).await;

        let mut events = events;
        let mut count: u64 = 0;
        loop {
            match events.next().await {
                Some(event) => {
                    let tag = match &event {
                        RadioEvent::Frequency(_) => "frequency",
                        RadioEvent::Mode(_, _) => "mode",
                        RadioEvent::Scope(_) => "scope",
                        RadioEvent::SignalMeter(_) => "signal_meter",
                        RadioEvent::RfPower(_) => "rf_power",
                        RadioEvent::Swr(_) => "swr",
                        RadioEvent::Alc(_) => "alc",
                    };
                    let json = match serde_json::to_string(&event) {
                        Ok(j) => j,
                        Err(e) => {
                            tracing::error!(cid, "failed to serialize radio event: {e}");
                            continue;
                        }
                    };
                    count += 1;
                    if count <= 20 || tag != "scope" {
                        tracing::debug!(cid, count, tag, "sending radio event");
                    }
                    if let Err(e) = dc.send_text(json).await {
                        tracing::debug!(cid, "radio channel send failed: {e}");
                        break;
                    }
                }
                None => {
                    tracing::info!(cid, "radio event stream ended");
                    break;
                }
            }
        }
        tracing::info!(cid, "radio data channel closed");
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wait for the data channel to open using the `on_open` callback.
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

/// Poll until the data channel reports `Open` state.
///
/// The `on_open` callback can fire before the SCTP association is fully
/// established, causing the first `send_text` to fail. This busy-waits
/// (with a short sleep) until `ready_state()` confirms the channel is open.
async fn wait_for_ready(dc: &Arc<webrtc::data_channel::RTCDataChannel>) {
    for _ in 0..50 {
        if dc.ready_state() == RTCDataChannelState::Open {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tracing::warn!("data channel did not reach Open state after 1s");
}
