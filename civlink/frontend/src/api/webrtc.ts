import type { SignalingClient } from "./signaling";

export async function createPeerConnection(
  signaling: SignalingClient,
): Promise<RTCPeerConnection> {
  const pc = new RTCPeerConnection({
    iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
  });

  pc.onicecandidate = (event) => {
    if (event.candidate) {
      signaling.send({ type: "ice-candidate", payload: event.candidate });
    }
  };

  // TODO: handle remote tracks (audio from radio)
  // TODO: create data channel for rig control

  return pc;
}
