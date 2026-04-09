// Copyright SM6WJM 2026

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;

use crate::app_state::AppState;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    // TODO: WebRTC signaling (offer/answer/ICE exchange)
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                tracing::debug!("signaling message: {text}");
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
