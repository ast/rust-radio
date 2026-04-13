// Copyright SM6WJM 2026

mod commands;
mod handshake;
mod message;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnection;

use crate::app_state::AppState;
use crate::radio::RadioHandle;
use crate::webrtc_transport;

use message::SignalingMessage;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!("WebSocket upgrade request");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let sid = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    tracing::info!(sid, "WebSocket connection established, waiting for offer");

    let Some(offer) = handshake::wait_for_offer(&mut socket).await else {
        tracing::warn!(sid, "client disconnected before sending offer");
        return;
    };

    let audio_rx = state.audio.as_ref().map(|a| a.subscribe());
    let pc = match webrtc_transport::create_peer_connection(audio_rx).await {
        Ok(pc) => pc,
        Err(e) => {
            tracing::error!(sid, "failed to create peer connection: {e}");
            return;
        }
    };

    let radio_events = subscribe_radio_events(state.radio.as_ref()).await;

    if let Err(e) = handshake::negotiate_answer(&pc, &mut socket, offer).await {
        tracing::error!(sid, "SDP negotiation failed: {e}");
        return;
    }

    let ice_rx = attach_ice_forwarder(&pc);
    attach_state_logger(&pc, sid);

    run_session(sid, &mut socket, &pc, state.radio.as_ref(), radio_events, ice_rx).await;

    if let Err(e) = pc.close().await {
        tracing::error!(sid, "failed to close peer connection: {e}");
    }
    tracing::info!(sid, "signaling session ended");
}

/// Subscribe to the radio event stream and kick off an initial state read so a
/// fresh client immediately receives current frequency/mode.
async fn subscribe_radio_events(
    radio: Option<&RadioHandle>,
) -> Option<impl Stream<Item = sidebridge::RadioEvent> + Unpin> {
    let radio = radio?;
    let events = radio.event_stream();
    if let Err(e) = radio.read_initial_state().await {
        tracing::warn!("failed to read initial radio state: {e}");
    }
    Some(events)
}

/// Wire up a callback that forwards server-side ICE candidates onto an mpsc
/// channel the session loop can pull from.
fn attach_ice_forwarder(pc: &RTCPeerConnection) -> mpsc::Receiver<RTCIceCandidateInit> {
    let (ice_tx, ice_rx) = mpsc::channel::<RTCIceCandidateInit>(32);
    pc.on_ice_candidate(Box::new(move |candidate| {
        let tx = ice_tx.clone();
        Box::pin(async move {
            let Some(c) = candidate else { return };
            match c.to_json() {
                Ok(init) => {
                    tracing::debug!("server ICE candidate: {}", init.candidate);
                    let _ = tx.send(init).await;
                }
                Err(e) => tracing::error!("failed to serialize ICE candidate: {e}"),
            }
        })
    }));
    ice_rx
}

fn attach_state_logger(pc: &RTCPeerConnection, sid: u64) {
    pc.on_peer_connection_state_change(Box::new(move |s| {
        tracing::info!(sid, "peer connection state: {s}");
        Box::pin(async {})
    }));
}

/// The main select! loop. Returns when the client disconnects or a send fails.
async fn run_session<S>(
    sid: u64,
    socket: &mut WebSocket,
    pc: &RTCPeerConnection,
    radio: Option<&RadioHandle>,
    mut radio_events: Option<S>,
    mut ice_rx: mpsc::Receiver<RTCIceCandidateInit>,
) where
    S: Stream<Item = sidebridge::RadioEvent> + Unpin,
{
    loop {
        tokio::select! {
            msg = socket.recv() => {
                if !handle_client_message(msg, pc, radio, sid).await {
                    break;
                }
            }
            Some(candidate) = ice_rx.recv() => {
                if send_ice_candidate(socket, candidate).await.is_err() {
                    tracing::error!(sid, "failed to send ICE candidate to client");
                    break;
                }
            }
            Some(event) = next_radio_event(radio_events.as_mut()) => {
                if forward_radio_event(socket, event, sid).await.is_err() {
                    break;
                }
            }
        }
    }
}

/// Poll the radio event stream if present, otherwise stay pending forever so
/// `select!` ignores this branch.
async fn next_radio_event<S>(stream: Option<&mut S>) -> Option<sidebridge::RadioEvent>
where
    S: Stream<Item = sidebridge::RadioEvent> + Unpin,
{
    match stream {
        Some(s) => s.next().await,
        None => std::future::pending().await,
    }
}

/// Handle one websocket message from the client. Returns false to terminate
/// the session (disconnect/error).
async fn handle_client_message(
    msg: Option<Result<Message, axum::Error>>,
    pc: &RTCPeerConnection,
    radio: Option<&RadioHandle>,
    sid: u64,
) -> bool {
    match msg {
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<SignalingMessage>(&text) {
                Ok(SignalingMessage::IceCandidate(c)) => {
                    tracing::debug!("client ICE candidate: {}", c.candidate);
                    if let Err(e) = pc.add_ice_candidate(c).await {
                        tracing::error!("failed to add ICE candidate: {e}");
                    }
                }
                Ok(SignalingMessage::RadioCommand(cmd)) => {
                    tracing::info!(sid, "radio command: {cmd:?}");
                    if let Some(r) = radio {
                        commands::dispatch(r, cmd).await;
                    }
                }
                Ok(other) => tracing::warn!("unexpected signaling message: {other:?}"),
                Err(e) => tracing::error!("failed to parse signaling message: {e}"),
            }
            true
        }
        Some(Ok(Message::Close(_))) => {
            tracing::info!(sid, "WebSocket client disconnected");
            false
        }
        Some(Err(e)) => {
            tracing::error!(sid, "WebSocket error: {e}");
            false
        }
        None => {
            tracing::info!(sid, "WebSocket stream ended");
            false
        }
        _ => true,
    }
}

async fn send_ice_candidate(
    socket: &mut WebSocket,
    candidate: RTCIceCandidateInit,
) -> Result<(), ()> {
    let msg = SignalingMessage::IceCandidate(candidate);
    let json = serde_json::to_string(&msg).map_err(|_| ())?;
    socket.send(Message::Text(json.into())).await.map_err(|_| ())
}

async fn forward_radio_event(
    socket: &mut WebSocket,
    event: sidebridge::RadioEvent,
    sid: u64,
) -> Result<(), ()> {
    match &event {
        sidebridge::RadioEvent::Scope(_) => {} // too noisy
        other => tracing::debug!(sid, "radio event: {other:?}"),
    }
    let value = serde_json::to_value(&event).map_err(|e| {
        tracing::error!("failed to serialize radio event: {e}");
    })?;
    let msg = SignalingMessage::RadioEvent(value);
    let json = serde_json::to_string(&msg).map_err(|_| ())?;
    socket.send(Message::Text(json.into())).await.map_err(|_| {
        tracing::debug!("failed to send radio event, client disconnected");
    })
}
