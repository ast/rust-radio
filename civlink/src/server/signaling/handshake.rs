// Copyright SM6WJM 2026

use axum::extract::ws::{Message, WebSocket};
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use super::message::SignalingMessage;

/// Read from the socket until an `Offer` arrives. Returns `None` if the client
/// disconnects or sends something unparseable first.
pub async fn wait_for_offer(socket: &mut WebSocket) -> Option<RTCSessionDescription> {
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        match serde_json::from_str::<SignalingMessage>(&text) {
            Ok(SignalingMessage::Offer(offer)) => {
                tracing::info!("received SDP offer from client");
                return Some(offer);
            }
            Ok(other) => tracing::warn!("expected offer, got: {other:?}"),
            Err(e) => tracing::error!("failed to parse signaling message: {e}"),
        }
    }
    None
}

/// Complete the SDP exchange: set the remote offer, build an answer, set it
/// as the local description, and push it back to the client.
pub async fn negotiate_answer(
    pc: &RTCPeerConnection,
    socket: &mut WebSocket,
    offer: RTCSessionDescription,
) -> Result<(), String> {
    pc.set_remote_description(offer)
        .await
        .map_err(|e| format!("set_remote_description: {e}"))?;

    let answer = pc
        .create_answer(None)
        .await
        .map_err(|e| format!("create_answer: {e}"))?;

    pc.set_local_description(answer.clone())
        .await
        .map_err(|e| format!("set_local_description: {e}"))?;

    let msg = SignalingMessage::Answer(answer);
    let json = serde_json::to_string(&msg).map_err(|e| format!("serialize answer: {e}"))?;
    tracing::info!("sending SDP answer to client");
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|e| format!("send answer: {e}"))
}
