// Copyright SM6WJM 2026

mod commands;
mod error;
mod handshake;
mod message;

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::RequestExt;
use axum::extract::{Request, State};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnection;

use crate::app_state::AppState;
use crate::radio::RadioHandle;
use crate::webrtc_transport;

use error::SignalingError;
use message::SignalingMessage;

static SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

type RadioEventStream = Pin<Box<dyn Stream<Item = sidebridge::RadioEvent> + Send + Unpin>>;

fn token_from_uri(uri: &Uri) -> Option<String> {
    url::form_urlencoded::parse(uri.query()?.as_bytes())
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.into_owned())
}

pub async fn ws_handler(
    State(state): State<Arc<AppState>>,
    mut req: Request,
) -> Result<Response, StatusCode> {
    let Some(token) = token_from_uri(req.uri()) else {
        tracing::warn!("rejected /ws: missing token");
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(username) = state.sessions.username(&token).await else {
        tracing::warn!("rejected /ws: unknown token");
        return Err(StatusCode::UNAUTHORIZED);
    };
    tracing::info!(%username, "WebSocket upgrade request");
    let ws: WebSocketUpgrade = req
        .extract_parts()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(ws.on_upgrade(move |socket| async move {
        if let Some(session) = SignalingSession::establish(socket, state, username).await {
            session.run().await;
        }
    }))
}

struct SignalingSession {
    sid: u64,
    username: String,
    socket: WebSocket,
    pc: Arc<RTCPeerConnection>,
    state: Arc<AppState>,
    ice_rx: mpsc::Receiver<RTCIceCandidateInit>,
    radio_events: Option<RadioEventStream>,
}

impl SignalingSession {
    /// Perform the full setup dance: wait for the client's SDP offer, build a
    /// peer connection, subscribe to radio events, negotiate the answer, and
    /// wire the ICE / connection-state callbacks. Returns `None` if any step
    /// fails — errors are logged inside.
    async fn establish(
        mut socket: WebSocket,
        state: Arc<AppState>,
        username: String,
    ) -> Option<Self> {
        let sid = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        tracing::info!(sid, %username, "WebSocket connection established, waiting for offer");

        let Some(offer) = handshake::wait_for_offer(&mut socket).await else {
            tracing::warn!(sid, "client disconnected before sending offer");
            return None;
        };

        let audio_rx = state.audio.as_ref().map(|a| a.subscribe());
        let pc = match webrtc_transport::create_peer_connection(audio_rx).await {
            Ok(pc) => pc,
            Err(e) => {
                tracing::error!(sid, "failed to create peer connection: {e}");
                return None;
            }
        };

        let radio_events = match state.radio.as_ref() {
            Some(radio) => {
                if let Err(e) = radio.read_initial_state().await {
                    tracing::warn!("failed to read initial radio state: {e}");
                }
                Some(Box::pin(radio.event_stream()) as RadioEventStream)
            }
            None => None,
        };

        if let Err(e) = handshake::negotiate_answer(&pc, &mut socket, offer).await {
            tracing::error!(sid, "SDP negotiation failed: {e}");
            return None;
        }

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

        pc.on_peer_connection_state_change(Box::new(move |s| {
            tracing::info!(sid, "peer connection state: {s}");
            Box::pin(async {})
        }));

        Some(Self {
            sid,
            username,
            socket,
            pc,
            state,
            ice_rx,
            radio_events,
        })
    }

    /// Main select! loop. Returns when the client disconnects or a send fails.
    async fn run(mut self) {
        loop {
            tokio::select! {
                msg = self.socket.recv() => {
                    if !self.handle_client_message(msg).await {
                        break;
                    }
                }
                Some(candidate) = self.ice_rx.recv() => {
                    if let Err(e) = self.send_ice_candidate(candidate).await {
                        tracing::error!(self.sid, "send ICE candidate: {e}");
                        break;
                    }
                }
                Some(event) = Self::next_radio_event(self.radio_events.as_mut()) => {
                    if let Err(e) = self.forward_radio_event(event).await {
                        tracing::debug!(self.sid, "forward radio event: {e}");
                        break;
                    }
                }
            }
        }

        if let Err(e) = self.pc.close().await {
            tracing::error!(self.sid, "failed to close peer connection: {e}");
        }
        tracing::info!(self.sid, username = %self.username, "signaling session ended");
    }

    fn radio(&self) -> Option<&RadioHandle> {
        self.state.radio.as_ref()
    }

    /// Poll the radio event stream if present, otherwise stay pending forever
    /// so `select!` ignores this branch.
    async fn next_radio_event(stream: Option<&mut RadioEventStream>) -> Option<sidebridge::RadioEvent> {
        match stream {
            Some(s) => s.next().await,
            None => std::future::pending().await,
        }
    }

    /// Handle one websocket message. Returns false to terminate the session.
    async fn handle_client_message(
        &mut self,
        msg: Option<Result<Message, axum::Error>>,
    ) -> bool {
        match msg {
            Some(Ok(Message::Text(text))) => {
                match serde_json::from_str::<SignalingMessage>(&text) {
                    Ok(SignalingMessage::IceCandidate(c)) => {
                        tracing::debug!("client ICE candidate: {}", c.candidate);
                        if let Err(e) = self.pc.add_ice_candidate(c).await {
                            tracing::error!("failed to add ICE candidate: {e}");
                        }
                    }
                    Ok(SignalingMessage::RadioCommand(cmd)) => {
                        tracing::info!(self.sid, "radio command: {cmd:?}");
                        if let Some(r) = self.radio() {
                            commands::dispatch(r, cmd).await;
                        }
                    }
                    Ok(other) => tracing::warn!("unexpected signaling message: {other:?}"),
                    Err(e) => tracing::error!("failed to parse signaling message: {e}"),
                }
                true
            }
            Some(Ok(Message::Close(_))) => {
                tracing::info!(self.sid, "WebSocket client disconnected");
                false
            }
            Some(Err(e)) => {
                tracing::error!(self.sid, "WebSocket error: {e}");
                false
            }
            None => {
                tracing::info!(self.sid, "WebSocket stream ended");
                false
            }
            _ => true,
        }
    }

    async fn send_ice_candidate(
        &mut self,
        candidate: RTCIceCandidateInit,
    ) -> Result<(), SignalingError> {
        let msg = SignalingMessage::IceCandidate(candidate);
        let json = serde_json::to_string(&msg)?;
        self.socket.send(Message::Text(json.into())).await?;
        Ok(())
    }

    async fn forward_radio_event(
        &mut self,
        event: sidebridge::RadioEvent,
    ) -> Result<(), SignalingError> {
        match &event {
            sidebridge::RadioEvent::Scope(_) => {} // too noisy
            other => tracing::debug!(self.sid, "radio event: {other:?}"),
        }
        let value = serde_json::to_value(&event)?;
        let msg = SignalingMessage::RadioEvent(value);
        let json = serde_json::to_string(&msg)?;
        self.socket.send(Message::Text(json.into())).await?;
        Ok(())
    }
}
