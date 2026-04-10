// Copyright SM6WJM 2026

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use sidebridge::RadioEvent;
use tokio_stream::{Stream, StreamExt};
use webrtc::peer_connection::RTCPeerConnection;

static CONNECTION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Register a handler that waits for the client-created "radio" data channel
/// and streams all radio events as JSON on it.
///
/// The event stream must be subscribed **before** calling this function so
/// that events triggered between subscription and channel-open are buffered.
pub fn setup_radio_channel(
    pc: &Arc<RTCPeerConnection>,
    events: Box<dyn Stream<Item = RadioEvent> + Send + Unpin>,
) {
    let events = Arc::new(tokio::sync::Mutex::new(Some(events)));

    pc.on_data_channel(Box::new(move |dc| {
        let events = Arc::clone(&events);
        Box::pin(async move {
            if dc.label() != "radio" {
                return;
            }

            let cid = CONNECTION_COUNTER.fetch_add(1, Ordering::Relaxed);
            tracing::info!(cid, "radio data channel received from client");

            // Take the event stream (only the first "radio" channel gets it)
            let stream = events.lock().await.take();
            let Some(events) = stream else {
                tracing::warn!(cid, "radio event stream already taken");
                return;
            };

            tokio::spawn(async move {
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
        })
    }));
}
