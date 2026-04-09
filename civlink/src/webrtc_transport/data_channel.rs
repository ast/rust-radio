// Copyright SM6WJM 2026

use std::sync::Arc;

use serde::Serialize;
use sidebridge::{IcomRadio, Radio, ScopeFrame};
use tokio::sync::oneshot;
use tokio_stream::{Stream, StreamExt};
use webrtc::peer_connection::RTCPeerConnection;

use crate::Result;
use crate::error::CivlinkError;

/// Tagged message sent over the "radio" data channel.
#[derive(Serialize)]
#[serde(tag = "type")]
enum RadioMessage {
    #[serde(rename = "spectrum")]
    Spectrum(ScopeFrame),
    #[serde(rename = "frequency")]
    Frequency { hz: u64 },
}

/// Create a "radio" data channel and pump spectrum + frequency updates as
/// tagged JSON messages.
pub async fn setup_radio_channel(
    pc: &Arc<RTCPeerConnection>,
    radio: Arc<IcomRadio>,
    scope_stream: Box<dyn Stream<Item = ScopeFrame> + Send + Unpin>,
    freq_stream: Box<dyn Stream<Item = u64> + Send + Unpin>,
) -> Result<()> {
    let dc = pc
        .create_data_channel("radio", None)
        .await
        .map_err(|e| CivlinkError::WebRtc(format!("failed to create data channel: {e}")))?;

    let (open_tx, open_rx) = oneshot::channel::<()>();
    let open_tx = std::sync::Mutex::new(Some(open_tx));

    dc.on_open(Box::new(move || {
        if let Some(tx) = open_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        Box::pin(async {})
    }));

    let dc = dc.clone();
    tokio::spawn(async move {
        if open_rx.await.is_err() {
            return;
        }
        tracing::info!("radio data channel open");

        // Send initial frequency so the display is correct before the first tune
        match radio.frequency().await {
            Ok(hz) => {
                let msg = RadioMessage::Frequency { hz };
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = dc.send_text(json).await;
                }
            }
            Err(e) => tracing::warn!("failed to read initial frequency: {e}"),
        }

        pump(dc, scope_stream, freq_stream).await;
        tracing::info!("radio data channel closed");
    });

    Ok(())
}

async fn pump(
    dc: Arc<webrtc::data_channel::RTCDataChannel>,
    mut scope: Box<dyn Stream<Item = ScopeFrame> + Send + Unpin>,
    mut freq: Box<dyn Stream<Item = u64> + Send + Unpin>,
) {
    loop {
        let msg = tokio::select! {
            Some(frame) = scope.next() => RadioMessage::Spectrum(frame),
            Some(hz) = freq.next() => RadioMessage::Frequency { hz },
            else => break,
        };
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("failed to serialize radio message: {e}");
                continue;
            }
        };
        if dc.send_text(json).await.is_err() {
            break;
        }
    }
}
