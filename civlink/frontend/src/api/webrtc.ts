import { SignalingClient } from "./signaling";
import type { SignalingMessage } from "../types/messages";

export async function createPeerConnection(
  token: string,
): Promise<{ pc: RTCPeerConnection; audioElement: HTMLAudioElement }> {
  const signaling = new SignalingClient();

  const pc = new RTCPeerConnection({
    iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
  });

  // Create an audio element to play the remote audio
  const audioElement = new Audio();
  audioElement.autoplay = true;

  // Handle remote tracks (audio from radio)
  pc.ontrack = (event) => {
    console.log(`[webrtc] remote track received: kind=${event.track.kind}, id=${event.track.id}`);
    if (event.streams.length > 0) {
      audioElement.srcObject = event.streams[0];
      console.log("[webrtc] audio stream attached to audio element");
    }
  };

  // Send ICE candidates to server
  pc.onicecandidate = (event) => {
    if (event.candidate) {
      console.log(`[webrtc] sending ICE candidate: ${event.candidate.candidate}`);
      signaling.send({ type: "ice-candidate", payload: event.candidate.toJSON() });
    }
  };

  pc.oniceconnectionstatechange = () => {
    console.log(`[webrtc] ICE connection state: ${pc.iceConnectionState}`);
  };

  pc.onconnectionstatechange = () => {
    console.log(`[webrtc] connection state: ${pc.connectionState}`);
  };

  // Handle incoming signaling messages
  signaling.setOnMessage((msg: SignalingMessage) => {
    switch (msg.type) {
      case "answer":
        console.log("[webrtc] received SDP answer");
        pc.setRemoteDescription(new RTCSessionDescription(msg.payload));
        break;
      case "ice-candidate":
        console.log(`[webrtc] received ICE candidate: ${msg.payload.candidate}`);
        pc.addIceCandidate(new RTCIceCandidate(msg.payload));
        break;
      default:
        console.warn("[webrtc] unexpected message:", msg);
    }
  });

  // Connect signaling and create offer
  signaling.connect(token);

  // Wait for WebSocket to open before sending offer
  await new Promise<void>((resolve) => {
    const check = setInterval(() => {
      if (signaling.isOpen()) {
        clearInterval(check);
        resolve();
      }
    }, 50);
  });

  // We need to add a transceiver for receiving audio (recvonly)
  pc.addTransceiver("audio", { direction: "recvonly" });

  const offer = await pc.createOffer();
  await pc.setLocalDescription(offer);

  console.log("[webrtc] sending SDP offer");
  signaling.send({ type: "offer", payload: offer });

  return { pc, audioElement };
}
