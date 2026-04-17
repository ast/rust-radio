import { Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import {
  COLORMAP_NAMES,
  ColormapName,
  DEFAULT_COLORMAP,
  colormap,
} from "../colormap";
import { Waterfall } from "../waterfall";
import FrequencyTuner from "./FrequencyTuner";

interface Props {
  token: string;
  onLogout: () => void;
}

interface DemodState {
  offset_hz: number;
  mode: string;
  filter_low_hz: number;
  filter_high_hz: number;
}

interface Hello {
  center_hz: number;
  samplerate: number;
  fft_len: number;
  fft_rate_hz: number;
  demod: DemodState;
}

interface Viewport {
  startHz: number;
  stopHz: number;
}

type LinkState = "connecting" | "connected" | "disconnected" | "error";
type DotKind = "idle" | "ok" | "warn" | "bad";

interface PeerStats {
  path: "direct" | "stun" | "turn" | "—";
  rttMs: number | null;
  lossPct: number | null;
  jitterMs: number | null;
}

const THEMES = ["cyan", "phosphor", "amber"] as const;
type Theme = (typeof THEMES)[number];
const THEME_KEY = "sdrlink.theme";

const fullViewport = (h: Hello): Viewport => ({
  startHz: h.center_hz - h.samplerate / 2,
  stopHz: h.center_hz + h.samplerate / 2,
});

const clampViewport = (vp: Viewport, h: Hello): Viewport => {
  const lo = h.center_hz - h.samplerate / 2;
  const hi = h.center_hz + h.samplerate / 2;
  const start = Math.max(lo, Math.min(hi, vp.startHz));
  const stop = Math.max(lo, Math.min(hi, vp.stopHz));
  if (stop - start < 1) return fullViewport(h);
  return { startHz: start, stopHz: stop };
};

const linkDot: Record<LinkState, DotKind> = {
  connecting: "warn",
  connected: "ok",
  disconnected: "bad",
  error: "bad",
};

const audioDot = (state: string): DotKind => {
  if (state === "connected" || state === "completed") return "ok";
  if (state === "failed" || state === "closed" || state === "disconnected") return "bad";
  if (state === "checking" || state === "negotiating" || state === "new") return "warn";
  return "idle";
};

const analyseStats = (report: RTCStatsReport): Partial<PeerStats> => {
  const out: Partial<PeerStats> = {};
  let pair: any = null;
  report.forEach((s: any) => {
    if (s.type === "candidate-pair" && s.nominated && s.state === "succeeded") pair = s;
  });
  if (pair) {
    const local: any = report.get(pair.localCandidateId);
    const remote: any = report.get(pair.remoteCandidateId);
    const lt = local?.candidateType;
    const rt = remote?.candidateType;
    if (lt === "relay" || rt === "relay") out.path = "turn";
    else if (lt === "srflx" || rt === "srflx" || lt === "prflx" || rt === "prflx") out.path = "stun";
    else if (lt === "host" && rt === "host") out.path = "direct";
    if (typeof pair.currentRoundTripTime === "number") out.rttMs = pair.currentRoundTripTime * 1000;
  }
  report.forEach((s: any) => {
    if (s.type === "inbound-rtp" && s.kind === "audio") {
      if (typeof s.jitter === "number") out.jitterMs = s.jitter * 1000;
      const lost = s.packetsLost ?? 0;
      const recv = s.packetsReceived ?? 0;
      if (lost + recv > 0) out.lossPct = (lost / (lost + recv)) * 100;
    }
  });
  return out;
};

const SpectrumView: Component<Props> = (props) => {
  const [hello, setHello] = createSignal<Hello | null>(null);
  const [viewport, setViewport] = createSignal<Viewport | null>(null);
  const [frames, setFrames] = createSignal(0);
  const [link, setLink] = createSignal<LinkState>("connecting");
  const [audioState, setAudioState] = createSignal("idle");
  const [peer, setPeer] = createSignal<PeerStats>({
    path: "—",
    rttMs: null,
    lossPct: null,
    jitterMs: null,
  });
  const [cmap, setCmap] = createSignal<ColormapName>(DEFAULT_COLORMAP);
  const [theme, setTheme] = createSignal<Theme>(
    (localStorage.getItem(THEME_KEY) as Theme) || "cyan",
  );
  const [selection, setSelection] = createSignal<{ x0: number; x1: number } | null>(null);
  const [demod, setDemod] = createSignal<DemodState | null>(null);
  const [playing, setPlaying] = createSignal(false);
  const [muted, setMuted] = createSignal(false);

  let canvas: HTMLCanvasElement | undefined;
  let audioEl: HTMLAudioElement | undefined;
  let ws: WebSocket | undefined;
  let waterfall: Waterfall | undefined;
  let pc: RTCPeerConnection | undefined;
  let statsTimer: number | undefined;
  let history: Viewport[] = [];
  let dragOrigin: { clientX: number; rectLeft: number; rectWidth: number } | null = null;

  const applyTheme = (t: Theme) => {
    setTheme(t);
    localStorage.setItem(THEME_KEY, t);
    if (t === "cyan") document.documentElement.removeAttribute("data-theme");
    else document.documentElement.setAttribute("data-theme", t);
  };

  const send = (msg: unknown) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(msg));
  };

  const sendViewport = (vp: Viewport, h: Hello) => {
    if (!canvas) return;
    const span = vp.stopHz - vp.startHz;
    const hzPerBin = h.samplerate / h.fft_len;
    const maxPixels = Math.max(1, Math.floor(span / hzPerBin));
    const pixels = Math.max(
      64,
      Math.min(canvas.clientWidth || 1024, maxPixels, h.fft_len),
    );
    send({
      type: "SetViewport",
      payload: { start_hz: vp.startHz, stop_hz: vp.stopHz, pixels },
    });
  };

  const applyViewport = (vp: Viewport, recordHistory: boolean) => {
    const h = hello();
    if (!h) return;
    const clamped = clampViewport(vp, h);
    if (recordHistory) {
      const cur = viewport();
      if (cur) history.push(cur);
    }
    setViewport(clamped);
    sendViewport(clamped, h);
  };

  const onMouseDown = (e: MouseEvent) => {
    if (e.button !== 0 || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    dragOrigin = { clientX: e.clientX, rectLeft: rect.left, rectWidth: rect.width };
    const x = e.clientX - rect.left;
    setSelection({ x0: x, x1: x });
  };

  const onMouseMove = (e: MouseEvent) => {
    if (!dragOrigin) return;
    const x = Math.max(0, Math.min(dragOrigin.rectWidth, e.clientX - dragOrigin.rectLeft));
    const sel = selection();
    if (sel) setSelection({ x0: sel.x0, x1: x });
  };

  const onMouseUp = (e: MouseEvent) => {
    if (!dragOrigin) return;
    const origin = dragOrigin;
    dragOrigin = null;
    const sel = selection();
    setSelection(null);
    if (!sel) return;
    const h = hello();
    const vp = viewport();
    if (!h || !vp) return;
    const x = Math.max(0, Math.min(origin.rectWidth, e.clientX - origin.rectLeft));
    const xMin = Math.min(sel.x0, x);
    const xMax = Math.max(sel.x0, x);
    if (xMax - xMin < 4) {
      const span = vp.stopHz - vp.startHz;
      const clickHz = vp.startHz + ((xMin + xMax) / 2 / origin.rectWidth) * span;
      const offset = clickHz - h.center_hz;
      const d = demod();
      if (d) setDemod({ ...d, offset_hz: offset });
      send({ type: "SetDemodOffset", payload: { offset_hz: offset } });
      return;
    }
    const span = vp.stopHz - vp.startHz;
    applyViewport(
      {
        startHz: vp.startHz + (xMin / origin.rectWidth) * span,
        stopHz: vp.startHz + (xMax / origin.rectWidth) * span,
      },
      true,
    );
  };

  const onWheel = (e: WheelEvent) => {
    e.preventDefault();
    const h = hello();
    const vp = viewport();
    const d = demod();
    if (!h || !vp || !d) return;
    const span = vp.stopHz - vp.startHz;
    const base = span / 200;
    const coarse = e.shiftKey ? 10 : 1;
    const dir = e.deltaY < 0 ? 1 : -1;
    const step = base * coarse * dir;
    const halfBand = h.samplerate / 2;
    const next = Math.max(-halfBand, Math.min(halfBand, d.offset_hz + step));
    if (next === d.offset_hz) return;
    setDemod({ ...d, offset_hz: next });
    send({ type: "SetDemodOffset", payload: { offset_hz: next } });
  };

  const onContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    const prev = history.pop();
    if (prev) applyViewport(prev, false);
  };

  const onDoubleClick = () => {
    const h = hello();
    if (!h) return;
    history = [];
    applyViewport(fullViewport(h), false);
  };

  const drawFrame = (view: DataView) => {
    if (!waterfall) return;
    const pixels = view.getUint16(0, true);
    if (pixels === 0) return;
    const max = new Uint8Array(view.buffer, 2 + pixels, pixels);
    waterfall.pushRow(max);
  };

  const startWebrtc = async () => {
    setAudioState("negotiating");
    pc = new RTCPeerConnection({
      iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
    });

    pc.ontrack = (e) => {
      if (audioEl && e.streams[0]) {
        audioEl.srcObject = e.streams[0];
        audioEl.play().catch((err) => {
          setAudioState(`autoplay blocked (${err.message})`);
        });
      }
    };
    pc.onconnectionstatechange = () => {
      if (pc) setAudioState(pc.connectionState);
    };
    pc.onicecandidate = (e) => {
      if (e.candidate) send({ type: "IceCandidate", payload: e.candidate.toJSON() });
    };

    const dc = pc.createDataChannel("spectrum", { ordered: false, maxRetransmits: 0 });
    dc.binaryType = "arraybuffer";
    dc.onmessage = (e) => {
      setFrames((n) => n + 1);
      drawFrame(new DataView(e.data as ArrayBuffer));
    };

    pc.addTransceiver("audio", { direction: "recvonly" });
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    send({ type: "Offer", payload: offer });

    statsTimer = window.setInterval(async () => {
      if (!pc) return;
      try {
        const report = await pc.getStats();
        setPeer((prev) => ({ ...prev, ...analyseStats(report) }));
      } catch {
        /* ignore */
      }
    }, 1000);
  };

  onMount(() => {
    if (canvas) waterfall = new Waterfall(canvas, colormap(cmap()));

    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/api/ws?token=${encodeURIComponent(props.token)}`;
    ws = new WebSocket(url);

    ws.onopen = () => setLink("connected");
    ws.onclose = () => setLink("disconnected");
    ws.onerror = () => setLink("error");

    ws.onmessage = async (e) => {
      const msg = JSON.parse(e.data);
      switch (msg.type) {
        case "Hello": {
          const h = msg.payload as Hello;
          setHello(h);
          setDemod(h.demod);
          const vp = fullViewport(h);
          setViewport(vp);
          sendViewport(vp, h);
          await startWebrtc();
          break;
        }
        case "Answer":
          if (pc) await pc.setRemoteDescription(msg.payload);
          break;
        case "CenterChanged": {
          const h = hello();
          if (h) {
            const updated = { ...h, center_hz: msg.payload.hz };
            setHello(updated);
            history = [];
            const next = fullViewport(updated);
            setViewport(next);
            sendViewport(next, updated);
          }
          break;
        }
        case "DemodChanged":
          setDemod(msg.payload as DemodState);
          break;
        case "IceCandidate":
          if (pc) {
            try {
              await pc.addIceCandidate(msg.payload);
            } catch (err) {
              console.warn("addIceCandidate failed", err);
            }
          }
          break;
      }
    };

    const resizeObserver = new ResizeObserver(() => {
      const h = hello();
      const v = viewport();
      if (h && v) sendViewport(v, h);
    });
    if (canvas) resizeObserver.observe(canvas);

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);

    onCleanup(() => {
      resizeObserver.disconnect();
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      if (statsTimer) clearInterval(statsTimer);
      pc?.close();
      ws?.close();
    });
  });

  const fmtMs = (v: number | null) => (v == null ? "—" : `${v.toFixed(0)}ms`);
  const fmtPct = (v: number | null) => (v == null ? "—" : `${v.toFixed(2)}%`);

  return (
    <div class="spectrum-view">
      <div class="topbar">
        <div class="stat stat--link">
          <span class="stat__label">link</span>
          <span class="stat__value">
            <span class={`dot dot--${linkDot[link()]}`} /> {link()}
          </span>
        </div>

        <div class="stat stat--audio">
          <span class="stat__label">audio</span>
          <span class="stat__value">
            <span class={`dot dot--${audioDot(audioState())}`} /> {audioState()}
            <span class="sep">·</span>
            {peer().path}
            <Show when={peer().rttMs != null}>
              <span class="sep">·</span>
              rtt {fmtMs(peer().rttMs)}
            </Show>
            <Show when={peer().lossPct != null}>
              <span class="sep">·</span>
              loss {fmtPct(peer().lossPct)}
            </Show>
            <Show when={peer().jitterMs != null}>
              <span class="sep">·</span>
              jit {fmtMs(peer().jitterMs)}
            </Show>
          </span>
        </div>

        <Show when={hello()}>
          {(h) => (
            <div class="stat stat--rx">
              <span class="stat__label">rx</span>
              <span class="stat__value">
                {(h().center_hz / 1e6).toFixed(3)} MHz
                <span class="sep">·</span>
                {(h().samplerate / 1000).toFixed(0)} kHz
                <span class="sep">·</span>
                {h().fft_len} bins
                <span class="sep">@</span>
                {h().fft_rate_hz} Hz
              </span>
            </div>
          )}
        </Show>

        <div class="stat stat--frames">
          <span class="stat__label">frames</span>
          <span class="stat__value">{frames().toLocaleString()}</span>
        </div>

        <div class="topbar__right">
          <button class="btn" onClick={props.onLogout}>logout</button>
        </div>
      </div>

      <div class="waterfall-wrap">
        <canvas
          ref={canvas}
          height={1024}
          class="waterfall"
          onMouseDown={onMouseDown}
          onWheel={onWheel}
          onContextMenu={onContextMenu}
          onDblClick={onDoubleClick}
        />
        <Show when={selection()}>
          {(s) => (
            <div
              class="selection-rect"
              style={{
                left: `${Math.min(s().x0, s().x1)}px`,
                width: `${Math.abs(s().x1 - s().x0)}px`,
              }}
            />
          )}
        </Show>
        <Show when={viewport()}>
          {(vp) => (
            <div class="overlay overlay--viewport">
              {(vp().startHz / 1e6).toFixed(3)} – {(vp().stopHz / 1e6).toFixed(3)} MHz
              <span class="sep">·</span>
              span {((vp().stopHz - vp().startHz) / 1e3).toFixed(1)} kHz
              {history.length > 0 && ` · ${history.length} back`}
            </div>
          )}
        </Show>
        {(() => {
          const h = hello();
          const vp = viewport();
          const d = demod();
          if (!h || !vp || !d) return null;
          const span = vp.stopHz - vp.startHz;
          const demodHz = h.center_hz + d.offset_hz;
          const loHz = demodHz + d.filter_low_hz;
          const hiHz = demodHz + d.filter_high_hz;
          if (hiHz < vp.startHz || loHz > vp.stopHz) return null;
          const leftPct = ((loHz - vp.startHz) / span) * 100;
          const widthPct = ((hiHz - loHz) / span) * 100;
          const centerPct = ((demodHz - vp.startHz) / span) * 100;
          return (
            <>
              <div class="filter-band" style={{ left: `${leftPct}%`, width: `${widthPct}%` }} />
              <div class="filter-center" style={{ left: `${centerPct}%` }} />
            </>
          );
        })()}
        <Show when={demod() && hello()}>
          <div class="overlay overlay--demod">
            demod {((hello()!.center_hz + demod()!.offset_hz) / 1e6).toFixed(3)} MHz
            <span class="sep">·</span>
            {demod()!.mode.toUpperCase()} {(demod()!.filter_low_hz / 1e3).toFixed(1)}…
            {(demod()!.filter_high_hz / 1e3).toFixed(1)} kHz
          </div>
        </Show>
      </div>

      <audio
        ref={audioEl}
        autoplay
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onVolumeChange={() => audioEl && setMuted(audioEl.muted)}
        style={{ display: "none" }}
      />

      <div class="controls">
        <div class="group">
          <div class="group__label">audio</div>
          <div class="group__body">
            <button
              class="btn"
              type="button"
              onClick={() => {
                if (!audioEl) return;
                if (audioEl.paused) audioEl.play().catch(() => {});
                else audioEl.pause();
              }}
              title={playing() ? "pause" : "play"}
            >
              {playing() ? "⏸" : "▶"}
            </button>
            <button
              class="btn"
              type="button"
              onClick={() => audioEl && (audioEl.muted = !audioEl.muted)}
              title={muted() ? "unmute" : "mute"}
            >
              {muted() ? "🔇" : "🔊"}
            </button>
          </div>
        </div>

        <div class="group">
          <div class="group__label">tuning</div>
          <div class="group__body">
            <Show when={hello()}>
              {(h) => (
                <FrequencyTuner
                  hz={h().center_hz}
                  onTune={(hz) => {
                    setHello({ ...h(), center_hz: hz });
                    send({ type: "SetCenter", payload: { hz } });
                  }}
                />
              )}
            </Show>
          </div>
        </div>

        <div class="group">
          <div class="group__label">demod</div>
          <div class="group__body">
            <select
              class="select"
              value={demod()?.mode ?? "fm"}
              onChange={(e) =>
                send({ type: "SetDemodMode", payload: { mode: e.currentTarget.value } })
              }
            >
              <option value="fm">FM</option>
              <option value="nfm">NFM</option>
              <option value="usb">USB</option>
              <option value="lsb">LSB</option>
            </select>
          </div>
        </div>

        <div class="group">
          <div class="group__label">colormap</div>
          <div class="group__body">
            <select
              class="select"
              value={cmap()}
              onChange={(e) => {
                const name = e.currentTarget.value as ColormapName;
                setCmap(name);
                waterfall?.setColormap(colormap(name));
              }}
            >
              {COLORMAP_NAMES.map((n) => (
                <option value={n}>{n}</option>
              ))}
            </select>
          </div>
        </div>

        <div class="group">
          <div class="group__label">theme</div>
          <div class="group__body">
            <select
              class="select"
              value={theme()}
              onChange={(e) => applyTheme(e.currentTarget.value as Theme)}
            >
              {THEMES.map((t) => (
                <option value={t}>{t}</option>
              ))}
            </select>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SpectrumView;
