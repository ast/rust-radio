import { type Component, createSignal, onMount, onCleanup } from "solid-js";
import VfoDisplay from "./VfoDisplay";
import Spectrum from "./Spectrum";
import AudioControls from "./AudioControls";
import { createPeerConnection, type PeerConnectionResult } from "../api/webrtc";

interface RadioPanelProps {
  token: string;
}

const RadioPanel: Component<RadioPanelProps> = (props) => {
  const [connected, setConnected] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [audioEl, setAudioEl] = createSignal<HTMLAudioElement | undefined>();
  const [audioBlocked, setAudioBlocked] = createSignal(false);
  const [bins, setBins] = createSignal<number[]>([]);
  const [frequency, setFrequency] = createSignal(0);
  const [mode, setMode] = createSignal("Usb");
  const [centerHz, setCenterHz] = createSignal(0);
  const [spanHz, setSpanHz] = createSignal(0);
  let conn: PeerConnectionResult | undefined;

  const disconnect = () => {
    console.log("[radio-panel] disconnecting");
    conn?.signaling.close();
    conn?.pc.close();
    conn = undefined;
  };

  const connect = async () => {
    disconnect();
    setConnected(false);
    setError(null);
    try {
      console.log("[radio-panel] establishing WebRTC connection");
      const result = await createPeerConnection(props.token);
      conn = result;
      setAudioEl(result.audioElement);

      // Try to start audio — may be blocked by autoplay policy
      result.audioElement.play().then(() => {
        setAudioBlocked(false);
      }).catch(() => {
        console.warn("[radio-panel] autoplay blocked, user interaction required");
        setAudioBlocked(true);
      });

      result.onScopeFrame((frame) => {
        setBins(frame.bins);
        setCenterHz(frame.center_hz);
        setSpanHz(frame.span_hz);
      });

      result.onFrequency((hz) => {
        setFrequency(hz);
      });

      result.onMode((m) => {
        setMode(m);
      });

      result.pc.onconnectionstatechange = () => {
        console.log(`[radio-panel] connection state: ${result.pc.connectionState}`);
        setConnected(result.pc.connectionState === "connected");
        if (result.pc.connectionState === "failed") {
          setError("Connection failed");
        }
      };
    } catch (e) {
      console.error("[radio-panel] failed to connect:", e);
      setError("Failed to establish connection");
    }
  };

  onMount(connect);

  // Clean up on beforeunload (page reload/close) and component unmount
  const onBeforeUnload = () => disconnect();
  window.addEventListener("beforeunload", onBeforeUnload);

  onCleanup(() => {
    window.removeEventListener("beforeunload", onBeforeUnload);
    disconnect();
  });

  const resumeAudio = () => {
    const el = audioEl();
    if (el) {
      el.play().then(() => setAudioBlocked(false)).catch(() => {});
    }
  };

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
      <VfoDisplay
        frequency={frequency()}
        mode={mode()}
        onTune={(hz) => conn?.sendCommand({ type: "set_frequency", data: hz })}
        onModeChange={(m) => conn?.sendCommand({ type: "set_mode", data: m })}
      />
      <Spectrum
        bins={bins()}
        centerHz={centerHz()}
        spanHz={spanHz()}
        onClickFrequency={(hz) => conn?.sendCommand({ type: "set_frequency", data: hz })}
      />
      {audioBlocked() ? (
        <button class="audio-start-btn" onClick={resumeAudio}>Start Audio</button>
      ) : (
        <AudioControls audioElement={audioEl()} />
      )}
    </div>
  );
};

export default RadioPanel;
