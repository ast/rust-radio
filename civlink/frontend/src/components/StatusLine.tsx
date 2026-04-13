import { type Component, Show } from "solid-js";
import type { ConnectionStats } from "../api/stats";

interface StatusLineProps {
  stats: ConnectionStats;
}

const formatRtt = (ms: number | null): string =>
  ms === null ? "—" : `${ms.toFixed(0)} ms`;

const formatKbps = (kbps: number): string =>
  kbps < 10 ? kbps.toFixed(1) : kbps.toFixed(0);

const formatPath = (s: ConnectionStats): string => {
  switch (s.path) {
    case "relay":
      return "TURN";
    case "direct-host":
      return "direct (LAN)";
    case "direct-srflx":
      return "direct (STUN)";
    default:
      return "—";
  }
};

const formatCodec = (mime: string | null): string => {
  if (!mime) return "—";
  return mime.replace(/^audio\//i, "").toUpperCase();
};

const StatusLine: Component<StatusLineProps> = (props) => {
  return (
    <div class={`status-line quality-${props.stats.quality}`}>
      <span class="status-line-dot" />
      <span class="status-line-field">
        <span class="status-line-label">RTT</span>
        <span class="status-line-value">{formatRtt(props.stats.rttMs)}</span>
      </span>
      <span class="status-line-field">
        <span class="status-line-label">↓</span>
        <span class="status-line-value">{formatKbps(props.stats.kbpsIn)} kb/s</span>
      </span>
      <span class="status-line-field">
        <span class="status-line-label">↑</span>
        <span class="status-line-value">{formatKbps(props.stats.kbpsOut)} kb/s</span>
      </span>
      <Show when={props.stats.jitterMs !== null}>
        <span class="status-line-field">
          <span class="status-line-label">jit</span>
          <span class="status-line-value">{props.stats.jitterMs!.toFixed(1)} ms</span>
        </span>
      </Show>
      <span class="status-line-field">
        <span class="status-line-label">loss</span>
        <span class="status-line-value">{props.stats.lossPct.toFixed(1)}%</span>
      </span>
      <span class="status-line-field">
        <span class="status-line-label">path</span>
        <span class="status-line-value">{formatPath(props.stats)}</span>
      </span>
      <span class="status-line-field">
        <span class="status-line-label">codec</span>
        <span class="status-line-value">{formatCodec(props.stats.codec)}</span>
      </span>
    </div>
  );
};

export default StatusLine;
