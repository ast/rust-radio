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
    tracing::info!("WebSocket upgrade request");
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket connection established");
    // TODO: WebRTC signaling (offer/answer/ICE exchange)
    while let Some(result) = socket.recv().await {
        match result {
            Ok(Message::Text(text)) => {
                tracing::debug!("signaling message: {text}");
            }
            Ok(Message::Close(_)) => {
                tracing::info!("WebSocket client disconnected");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("WebSocket error: {e}");
                break;
            }
        }
    }
}
