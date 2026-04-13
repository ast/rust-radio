import { type Accessor, createSignal } from "solid-js";

export type ConnectionPath = "direct-host" | "direct-srflx" | "relay" | "unknown";
export type ConnectionQuality = "good" | "fair" | "poor" | "unknown";

export interface ConnectionStats {
  rttMs: number | null;
  kbpsIn: number;
  kbpsOut: number;
  jitterMs: number | null;
  lossPct: number;
  path: ConnectionPath;
  localCandType: string | null;
  remoteCandType: string | null;
  codec: string | null;
  quality: ConnectionQuality;
}

export interface StatsSample {
  t: number;
  kbpsIn: number;
  kbpsOut: number;
  rttMs: number | null;
  lossPct: number;
}

const EMPTY: ConnectionStats = {
  rttMs: null,
  kbpsIn: 0,
  kbpsOut: 0,
  jitterMs: null,
  lossPct: 0,
  path: "unknown",
  localCandType: null,
  remoteCandType: null,
  codec: null,
  quality: "unknown",
};

const HISTORY_LEN = 120;

function derivePath(local: string | null, remote: string | null): ConnectionPath {
  if (local === "relay" || remote === "relay") return "relay";
  if (local === "host" && remote === "host") return "direct-host";
  if (local && remote) return "direct-srflx";
  return "unknown";
}

function deriveQuality(rttMs: number | null, lossPct: number): ConnectionQuality {
  if (rttMs === null) return "unknown";
  if (rttMs < 100 && lossPct < 1) return "good";
  if (rttMs < 250 && lossPct < 3) return "fair";
  return "poor";
}

export class StatsCollector {
  private readonly pc: RTCPeerConnection;
  private readonly intervalMs: number;
  private timer: ReturnType<typeof setInterval> | null = null;

  private prevBytesIn = 0;
  private prevBytesOut = 0;
  private prevTs = 0;
  private prevPacketsLost = 0;
  private prevPacketsReceived = 0;

  readonly current: Accessor<ConnectionStats>;
  readonly history: Accessor<StatsSample[]>;

  private readonly setCurrent: (s: ConnectionStats) => void;
  private readonly setHistory: (fn: (prev: StatsSample[]) => StatsSample[]) => void;

  constructor(pc: RTCPeerConnection, intervalMs = 1000) {
    this.pc = pc;
    this.intervalMs = intervalMs;
    const [current, setCurrent] = createSignal<ConnectionStats>(EMPTY);
    const [history, setHistory] = createSignal<StatsSample[]>([]);
    this.current = current;
    this.history = history;
    this.setCurrent = setCurrent;
    this.setHistory = setHistory;
  }

  start(): void {
    if (this.timer !== null) return;
    this.timer = setInterval(() => this.poll(), this.intervalMs);
  }

  stop(): void {
    if (this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  private async poll(): Promise<void> {
    let report: RTCStatsReport;
    try {
      report = await this.pc.getStats();
    } catch {
      return;
    }

    let pair: any = null;
    let localCand: any = null;
    let remoteCand: any = null;
    let inbound: any = null;
    let codec: any = null;
    const codecs = new Map<string, any>();
    const candidates = new Map<string, any>();

    report.forEach((v: any) => {
      switch (v.type) {
        case "candidate-pair":
          if (v.nominated && v.state === "succeeded") pair = v;
          else if (!pair && v.selected) pair = v;
          break;
        case "local-candidate":
        case "remote-candidate":
          candidates.set(v.id, v);
          break;
        case "inbound-rtp":
          if (v.kind === "audio") inbound = v;
          break;
        case "codec":
          codecs.set(v.id, v);
          break;
      }
    });

    if (pair) {
      localCand = candidates.get(pair.localCandidateId) ?? null;
      remoteCand = candidates.get(pair.remoteCandidateId) ?? null;
    }
    if (inbound?.codecId) codec = codecs.get(inbound.codecId) ?? null;

    const now = pair?.timestamp ?? performance.now();
    const bytesIn = pair?.bytesReceived ?? 0;
    const bytesOut = pair?.bytesSent ?? 0;

    let kbpsIn = 0;
    let kbpsOut = 0;
    if (this.prevTs > 0 && now > this.prevTs) {
      const dt = (now - this.prevTs) / 1000;
      kbpsIn = ((bytesIn - this.prevBytesIn) * 8) / 1000 / dt;
      kbpsOut = ((bytesOut - this.prevBytesOut) * 8) / 1000 / dt;
    }
    this.prevTs = now;
    this.prevBytesIn = bytesIn;
    this.prevBytesOut = bytesOut;

    let lossPct = 0;
    if (inbound) {
      const dLost = (inbound.packetsLost ?? 0) - this.prevPacketsLost;
      const dRecv = (inbound.packetsReceived ?? 0) - this.prevPacketsReceived;
      const total = dLost + dRecv;
      if (total > 0) lossPct = (dLost / total) * 100;
      this.prevPacketsLost = inbound.packetsLost ?? 0;
      this.prevPacketsReceived = inbound.packetsReceived ?? 0;
    }

    const rttMs =
      typeof pair?.currentRoundTripTime === "number"
        ? pair.currentRoundTripTime * 1000
        : null;
    const jitterMs =
      typeof inbound?.jitter === "number" ? inbound.jitter * 1000 : null;

    const localType = localCand?.candidateType ?? null;
    const remoteType = remoteCand?.candidateType ?? null;

    const stats: ConnectionStats = {
      rttMs,
      kbpsIn: Math.max(0, kbpsIn),
      kbpsOut: Math.max(0, kbpsOut),
      jitterMs,
      lossPct,
      path: derivePath(localType, remoteType),
      localCandType: localType,
      remoteCandType: remoteType,
      codec: codec?.mimeType ?? null,
      quality: deriveQuality(rttMs, lossPct),
    };

    this.setCurrent(stats);
    this.setHistory((prev) => {
      const next = prev.concat({
        t: now,
        kbpsIn: stats.kbpsIn,
        kbpsOut: stats.kbpsOut,
        rttMs: stats.rttMs,
        lossPct: stats.lossPct,
      });
      if (next.length > HISTORY_LEN) next.splice(0, next.length - HISTORY_LEN);
      return next;
    });
  }
}
