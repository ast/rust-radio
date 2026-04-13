import { type Component, Show, createEffect, createSignal, onMount, onCleanup } from "solid-js";
import VfoDisplay from "./VfoDisplay";
import Spectrum from "./Spectrum";
import AudioControls from "./AudioControls";
import StatusLine from "./StatusLine";
import StatsGraph from "./StatsGraph";
import { createPeerConnection, type PeerConnectionResult } from "../api/webrtc";
import { StatsCollector, type ConnectionStats, type StatsSample } from "../api/stats";
import type { ScopeFreq, ScopeMode } from "../types/radio";

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
  const [filter, setFilter] = createSignal(1);
  const [scopeFreq, setScopeFreq] = createSignal<ScopeFreq | undefined>();
  const [scopeMode, setScopeMode] = createSignal<ScopeMode>("center");
  const [fixedEdge, setFixedEdge] = createSignal(1);
  const [showStats, setShowStats] = createSignal(true);
  const [stats, setStats] = createSignal<ConnectionStats | null>(null);
  const [statsHistory, setStatsHistory] = createSignal<StatsSample[]>([]);
  let conn: PeerConnectionResult | undefined;
  let statsCollector: StatsCollector | undefined;

  const disconnect = () => {
    console.log("[radio-panel] disconnecting");
    statsCollector?.stop();
    statsCollector = undefined;
    setStats(null);
    setStatsHistory([]);
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
        setScopeFreq(frame.freq);
        setScopeMode(frame.freq.mode);
      });

      result.onFrequency((hz) => {
        setFrequency(hz);
      });

      result.onMode((m, f) => {
        setMode(m);
        setFilter(f);
      });

      result.pc.onconnectionstatechange = () => {
        console.log(`[radio-panel] connection state: ${result.pc.connectionState}`);
        setConnected(result.pc.connectionState === "connected");
        if (result.pc.connectionState === "failed") {
          setError("Connection failed");
        }
      };

      statsCollector = new StatsCollector(result.pc);
      statsCollector.start();
      createEffect(() => setStats(statsCollector!.current()));
      createEffect(() => setStatsHistory(statsCollector!.history()));
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
      <div class="header-bar">
        <span class="header-title">civlink</span>
        <div class="header-right">
          <button
            class="stats-toggle"
            classList={{ active: showStats() }}
            onClick={() => setShowStats((s) => !s)}
            title="Toggle connection stats"
          >
            stats
          </button>
          <div class="connection-status">
            {error() ? (
              <span class="status-error"><span class="status-dot" />{error()}</span>
            ) : connected() ? (
              <span class="status-connected"><span class="status-dot" />Connected</span>
            ) : (
              <span class="status-connecting"><span class="status-dot" />Connecting</span>
            )}
          </div>
        </div>
      </div>
      <Show when={showStats() && stats()}>
        <div class="stats-panel">
          <StatusLine stats={stats()!} />
          <StatsGraph history={statsHistory()} />
        </div>
      </Show>
      <VfoDisplay
        frequency={frequency()}
        mode={mode()}
        onTune={(hz) => conn?.sendCommand({ type: "set_frequency", data: hz })}
        onModeChange={(m) => conn?.sendCommand({ type: "set_mode", data: m })}
        onBandSelect={(hz, m) => {
          conn?.sendCommand({ type: "set_frequency", data: hz });
          conn?.sendCommand({ type: "set_mode", data: m });
        }}
      />
      <div class="spectrum-section">
        <Spectrum
          bins={bins()}
          scopeFreq={scopeFreq()}
          frequency={frequency()}
          mode={mode()}
          filter={filter()}
          scopeMode={scopeMode()}
          fixedEdge={fixedEdge()}
          onClickFrequency={(hz) => conn?.sendCommand({ type: "set_frequency", data: hz })}
          onSpanChange={(hz) => conn?.sendCommand({ type: "set_scope_span", data: hz })}
          onScopeModeChange={(m) => {
            setScopeMode(m);
            conn?.sendCommand({ type: "set_scope_mode", data: m });
          }}
          onFixedEdgeChange={(edge) => {
            setFixedEdge(edge);
            conn?.sendCommand({ type: "set_scope_fixed_edge", data: edge });
          }}
        />
      </div>
      <div class="controls-bar">
        {audioBlocked() ? (
          <button class="audio-start-btn" onClick={resumeAudio}>Start Audio</button>
        ) : (
          <AudioControls audioElement={audioEl()} />
        )}
      </div>
    </div>
  );
};

export default RadioPanel;
