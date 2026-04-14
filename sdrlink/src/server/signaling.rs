use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use crate::app_state::AppState;
use crate::config::SdrConfig;
use crate::sdr::Viewport;

/// Pre-WebRTC transport for spectrum frames. Once the WebRTC `spectrum` data
/// channel lands (plan step 6) this becomes signaling-only.
#[derive(Deserialize)]
pub struct WsParams {
    pub token: String,
}

#[derive(Serialize)]
#[serde(tag = "type")]
#[allow(clippy::enum_variant_names)]
enum ServerMessage {
    Hello {
        center_hz: f64,
        samplerate: u32,
        fft_len: u32,
        fft_rate_hz: u32,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    /// Set this client's spectrum viewport. Server clamps to passband.
    SetViewport {
        start_hz: f64,
        stop_hz: f64,
        pixels: u16,
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

    let hello = ServerMessage::Hello {
        center_hz: cfg.center_hz,
        samplerate: cfg.samplerate,
        fft_len: cfg.fft_len,
        fft_rate_hz: cfg.fft_rate_hz,
    };

    let hello_json = match serde_json::to_string(&hello) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("failed to serialize hello: {e}");
            return;
        }
    };

    if socket.send(Message::Text(hello_json.into())).await.is_err() {
        return;
    }

    let mut rx = state.sdr.subscribe_spectrum();

    // Per-client state: viewport + reusable decimation buffers so we don't
    // allocate on the hot path.
    let mut viewport: Option<Viewport> = None;
    let mut min_buf: Vec<u8> = Vec::new();
    let mut max_buf: Vec<u8> = Vec::new();
    let mut wire_buf: Vec<u8> = Vec::new();

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
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("ws receiver lagged {n} frames, continuing");
                }
                Err(RecvError::Closed) => {
                    tracing::info!("spectrum channel closed");
                    break;
                }
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::SetViewport { start_hz, stop_hz, pixels }) => {
                            viewport = build_viewport(&cfg, start_hz, stop_hz, pixels);
                            if let Some(vp) = viewport {
                                min_buf.resize(vp.pixels as usize, 0);
                                max_buf.resize(vp.pixels as usize, 0);
                                tracing::debug!(
                                    ?vp, start_hz, stop_hz, pixels,
                                    "viewport set"
                                );
                            } else {
                                tracing::warn!("invalid viewport: {start_hz}..{stop_hz} px={pixels}");
                            }
                        }
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

    tracing::info!("ws disconnected");
}

fn build_viewport(cfg: &SdrConfig, start_hz: f64, stop_hz: f64, pixels: u16) -> Option<Viewport> {
    Viewport::from_hz(
        start_hz,
        stop_hz,
        pixels,
        cfg.center_hz,
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
