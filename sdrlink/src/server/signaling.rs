use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::app_state::AppState;
use crate::config::SdrConfig;
use crate::sdr::Viewport;
use crate::webrtc_transport;

#[derive(Deserialize)]
pub struct WsParams {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
enum SignalingMessage {
    Hello {
        center_hz: f64,
        samplerate: u32,
        fft_len: u32,
        fft_rate_hz: u32,
    },
    SetViewport {
        start_hz: f64,
        stop_hz: f64,
        pixels: u16,
    },
    Offer(RTCSessionDescription),
    Answer(RTCSessionDescription),
    IceCandidate(RTCIceCandidateInit),
    SetCenter {
        hz: f64,
    },
    CenterChanged {
        hz: f64,
    },
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let Some(username) = state.sessions.username(&params.token).await else {
        tracing::warn!("ws rejected: unknown token");
        return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    };

    tracing::info!("ws connected: user '{}'", username);
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let cfg = state.sdr.config().clone();
    let mut current_center = state.sdr.center_hz();

    let hello = SignalingMessage::Hello {
        center_hz: current_center,
        samplerate: cfg.samplerate,
        fft_len: cfg.fft_len,
        fft_rate_hz: cfg.fft_rate_hz,
    };
    if !send_json(&mut socket, &hello).await {
        return;
    }

    let mut rx = state.sdr.subscribe_spectrum();
    let mut center_rx = state.sdr.subscribe_center();

    let mut viewport: Option<Viewport> = None;
    let mut min_buf: Vec<u8> = Vec::new();
    let mut max_buf: Vec<u8> = Vec::new();
    let mut wire_buf: Vec<u8> = Vec::new();

    let mut pc: Option<Arc<RTCPeerConnection>> = None;
    let (ice_tx, mut ice_rx) = mpsc::channel::<RTCIceCandidateInit>(32);

    loop {
        tokio::select! {
            frame = rx.recv() => match frame {
                Ok(pds) => {
                    let Some(vp) = viewport else { continue };
                    encode_frame(&vp, &pds, &mut min_buf, &mut max_buf, &mut wire_buf);
                    if socket.send(Message::Binary(wire_buf.clone().into())).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(n)) => tracing::warn!("ws lagged {n} frames"),
                Err(RecvError::Closed) => {
                    tracing::info!("spectrum channel closed");
                    break;
                }
            },
            Some(cand) = ice_rx.recv() => {
                let msg = SignalingMessage::IceCandidate(cand);
                if !send_json(&mut socket, &msg).await {
                    break;
                }
            },
            res = center_rx.recv() => match res {
                Ok(hz) => {
                    current_center = hz;
                    let msg = SignalingMessage::CenterChanged { hz };
                    if !send_json(&mut socket, &msg).await {
                        break;
                    }
                }
                Err(RecvError::Lagged(_)) => {}
                Err(RecvError::Closed) => break,
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<SignalingMessage>(&text) {
                        Ok(SignalingMessage::SetViewport { start_hz, stop_hz, pixels }) => {
                            viewport = build_viewport(&cfg, current_center, start_hz, stop_hz, pixels);
                            if let Some(vp) = viewport {
                                min_buf.resize(vp.pixels as usize, 0);
                                max_buf.resize(vp.pixels as usize, 0);
                                tracing::debug!(?vp, "viewport set");
                            } else {
                                tracing::warn!("invalid viewport: {start_hz}..{stop_hz} px={pixels}");
                            }
                        }
                        Ok(SignalingMessage::SetCenter { hz }) => {
                            if let Err(e) = state.sdr.set_center_hz(hz) {
                                tracing::error!("set_center_hz failed: {e}");
                            }
                        }
                        Ok(SignalingMessage::Offer(offer)) => {
                            match handle_offer(state.clone(), offer, ice_tx.clone()).await {
                                Ok((new_pc, answer)) => {
                                    pc = Some(new_pc);
                                    let msg = SignalingMessage::Answer(answer);
                                    if !send_json(&mut socket, &msg).await {
                                        break;
                                    }
                                }
                                Err(e) => tracing::error!("offer handling failed: {e}"),
                            }
                        }
                        Ok(SignalingMessage::IceCandidate(c)) => {
                            if let Some(p) = pc.as_ref()
                                && let Err(e) = p.add_ice_candidate(c).await
                            {
                                tracing::error!("add ice candidate: {e}");
                            }
                        }
                        Ok(_) => {}
                        Err(e) => tracing::warn!("bad client message: {e}"),
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(e)) => {
                    tracing::warn!("ws recv error: {e}");
                    break;
                }
                _ => {}
            },
        }
    }

    if let Some(p) = pc {
        let _ = p.close().await;
    }
    tracing::info!("ws disconnected");
}

async fn handle_offer(
    state: Arc<AppState>,
    offer: RTCSessionDescription,
    ice_tx: mpsc::Sender<RTCIceCandidateInit>,
) -> Result<(Arc<RTCPeerConnection>, RTCSessionDescription), String> {
    let audio_rx = state.sdr.subscribe_audio();
    let pc = webrtc_transport::create_peer_connection(audio_rx)
        .await
        .map_err(|e| format!("create pc: {e}"))?;

    pc.on_ice_candidate(Box::new(move |candidate| {
        let tx = ice_tx.clone();
        Box::pin(async move {
            let Some(c) = candidate else { return };
            if let Ok(init) = c.to_json() {
                let _ = tx.send(init).await;
            }
        })
    }));

    pc.on_peer_connection_state_change(Box::new(move |s| {
        tracing::info!("peer connection state: {s}");
        Box::pin(async {})
    }));

    pc.set_remote_description(offer)
        .await
        .map_err(|e| format!("set remote: {e}"))?;
    let answer = pc
        .create_answer(None)
        .await
        .map_err(|e| format!("create answer: {e}"))?;
    pc.set_local_description(answer.clone())
        .await
        .map_err(|e| format!("set local: {e}"))?;

    Ok((pc, answer))
}

async fn send_json(socket: &mut WebSocket, msg: &SignalingMessage) -> bool {
    match serde_json::to_string(msg) {
        Ok(s) => socket.send(Message::Text(s.into())).await.is_ok(),
        Err(e) => {
            tracing::error!("serialize signaling: {e}");
            false
        }
    }
}

fn build_viewport(
    cfg: &SdrConfig,
    center_hz: f64,
    start_hz: f64,
    stop_hz: f64,
    pixels: u16,
) -> Option<Viewport> {
    Viewport::from_hz(
        start_hz,
        stop_hz,
        pixels,
        center_hz,
        cfg.samplerate,
        cfg.fft_len as usize,
    )
}

/// Wire format: `[u16_le pixels][u8 min[pixels]][u8 max[pixels]]`.
fn encode_frame(
    vp: &Viewport,
    pds: &[u8],
    min_buf: &mut [u8],
    max_buf: &mut [u8],
    out: &mut Vec<u8>,
) {
    vp.decimate(pds, min_buf, max_buf);
    out.clear();
    out.reserve(2 + min_buf.len() + max_buf.len());
    out.extend_from_slice(&vp.pixels.to_le_bytes());
    out.extend_from_slice(min_buf);
    out.extend_from_slice(max_buf);
}
