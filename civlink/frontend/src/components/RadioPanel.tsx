import { type Component, createSignal, onMount, onCleanup } from "solid-js";
import VfoDisplay from "./VfoDisplay";
import Spectrum from "./Spectrum";
import AudioControls from "./AudioControls";
import { createPeerConnection } from "../api/webrtc";

interface RadioPanelProps {
  token: string;
}

const RadioPanel: Component<RadioPanelProps> = (props) => {
  const [connected, setConnected] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let pc: RTCPeerConnection | undefined;
  let audioEl: HTMLAudioElement | undefined;

  onMount(async () => {
    try {
      console.log("[radio-panel] establishing WebRTC connection");
      const result = await createPeerConnection(props.token);
      pc = result.pc;
      audioEl = result.audioElement;

      pc.onconnectionstatechange = () => {
        console.log(`[radio-panel] connection state: ${pc!.connectionState}`);
        setConnected(pc!.connectionState === "connected");
        if (pc!.connectionState === "failed") {
          setError("Connection failed");
        }
      };
    } catch (e) {
      console.error("[radio-panel] failed to connect:", e);
      setError("Failed to establish connection");
    }
  });

  onCleanup(() => {
    console.log("[radio-panel] cleaning up");
    pc?.close();
  });

  return (
    <div class="radio-panel">
      <div class="connection-status">
        {error() ? (
          <span class="status-error">{error()}</span>
        ) : connected() ? (
          <span class="status-connected">Connected</span>
        ) : (
          <span class="status-connecting">Connecting...</span>
        )}
      </div>
      <VfoDisplay frequency={14074000} mode="USB" />
      <Spectrum bins={[]} />
      <AudioControls />
    </div>
  );
};

export default RadioPanel;
