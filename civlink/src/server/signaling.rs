// Copyright SM6WJM 2026

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use sidebridge::RadioScope;

use crate::app_state::AppState;
use crate::webrtc_transport;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
enum SignalingMessage {
    #[serde(rename = "offer")]
    Offer(RTCSessionDescription),
    #[serde(rename = "answer")]
    Answer(RTCSessionDescription),
    #[serde(rename = "ice-candidate")]
    IceCandidate(RTCIceCandidateInit),
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("WebSocket upgrade request");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("WebSocket connection established, waiting for offer");

    // Wait for the client's offer
    let Some(offer) = wait_for_offer(&mut socket).await else {
        tracing::warn!("client disconnected before sending offer");
        return;
    };

    // Create peer connection (audio is optional)
    let audio_rx = state.audio.as_ref().map(|a| a.subscribe());
    let pc = match webrtc_transport::create_peer_connection(audio_rx).await {
        Ok(pc) => pc,
        Err(e) => {
            tracing::error!("failed to create peer connection: {e}");
            return;
        }
    };

    // Set up radio data channels if radio is connected
    if let Some(ref handle) = state.radio {
        let radio = handle.radio();
        let radio2 = handle.radio();
        let radio3 = handle.radio();

        if let Err(e) = webrtc_transport::data_channel::setup_state_channel(
            &pc,
            radio.clone(),
            move || radio.freq_stream(),
            move || radio2.mode_stream(),
        )
        .await
        {
            tracing::warn!("failed to set up state data channel: {e}");
        }

        if let Err(e) = webrtc_transport::data_channel::setup_spectrum_channel(
            &pc,
            move || radio3.scope_stream(),
        )
        .await
        {
            tracing::warn!("failed to set up spectrum data channel: {e}");
        }
    }

    // Set the remote description (client's offer)
    if let Err(e) = pc.set_remote_description(offer).await {
        tracing::error!("failed to set remote description: {e}");
        return;
    }

    // Create an answer
    let answer = match pc.create_answer(None).await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("failed to create answer: {e}");
            return;
        }
    };

    // Set the local description
    if let Err(e) = pc.set_local_description(answer.clone()).await {
        tracing::error!("failed to set local description: {e}");
        return;
    }

    // Send the answer back to the client
    let answer_msg = SignalingMessage::Answer(answer);
    if let Ok(json) = serde_json::to_string(&answer_msg) {
        tracing::info!("sending SDP answer to client");
        if socket.send(Message::Text(json.into())).await.is_err() {
            tracing::error!("failed to send answer");
            return;
        }
    }

    // Forward ICE candidates from server to client
    let (ice_tx, mut ice_rx) = tokio::sync::mpsc::channel::<RTCIceCandidateInit>(32);
    pc.on_ice_candidate(Box::new(move |candidate| {
        let tx = ice_tx.clone();
        Box::pin(async move {
            if let Some(c) = candidate {
                match c.to_json() {
                    Ok(init) => {
                        tracing::debug!("server ICE candidate: {}", init.candidate);
                        let _ = tx.send(init).await;
                    }
                    Err(e) => tracing::error!("failed to serialize ICE candidate: {e}"),
                }
            }
        })
    }));

    // Connection state logging
    pc.on_peer_connection_state_change(Box::new(move |state| {
        tracing::info!("peer connection state: {state}");
        Box::pin(async {})
    }));

    // Main signaling loop: handle incoming messages and forward ICE candidates
    loop {
        tokio::select! {
            // Incoming WebSocket message from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<SignalingMessage>(&text) {
                            Ok(SignalingMessage::IceCandidate(candidate)) => {
                                tracing::debug!("client ICE candidate: {}", candidate.candidate);
                                if let Err(e) = pc.add_ice_candidate(candidate).await {
                                    tracing::error!("failed to add ICE candidate: {e}");
                                }
                            }
                            Ok(other) => {
                                tracing::warn!("unexpected signaling message: {other:?}");
                            }
                            Err(e) => {
                                tracing::error!("failed to parse signaling message: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error: {e}");
                        break;
                    }
                    None => {
                        tracing::info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
            // Outgoing ICE candidate to client
            Some(candidate) = ice_rx.recv() => {
                let msg = SignalingMessage::IceCandidate(candidate);
                if let Ok(json) = serde_json::to_string(&msg) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        tracing::error!("failed to send ICE candidate to client");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup
    if let Err(e) = pc.close().await {
        tracing::error!("failed to close peer connection: {e}");
    }
    tracing::info!("signaling session ended");
}

async fn wait_for_offer(socket: &mut WebSocket) -> Option<RTCSessionDescription> {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<SignalingMessage>(&text) {
                Ok(SignalingMessage::Offer(offer)) => {
                    tracing::info!("received SDP offer from client");
                    return Some(offer);
                }
                Ok(other) => {
                    tracing::warn!("expected offer, got: {other:?}");
                }
                Err(e) => {
                    tracing::error!("failed to parse signaling message: {e}");
                }
            }
        }
    }
    None
}
