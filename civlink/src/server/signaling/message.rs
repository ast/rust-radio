// Copyright SM6WJM 2026

use serde::{Deserialize, Serialize};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

/// Wire format for all WebSocket signaling messages between the civlink
/// frontend and backend. Tagged by `type` so serde picks the variant.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SignalingMessage {
    #[serde(rename = "offer")]
    Offer(RTCSessionDescription),
    #[serde(rename = "answer")]
    Answer(RTCSessionDescription),
    #[serde(rename = "ice-candidate")]
    IceCandidate(RTCIceCandidateInit),
    /// Radio event forwarded as a pre-serialized JSON value.
    #[serde(rename = "radio-event")]
    RadioEvent(serde_json::Value),
    /// Radio command from the frontend.
    #[serde(rename = "radio-command")]
    RadioCommand(sidebridge::RadioCommand),
}
