import { Component, createSignal, onCleanup, onMount } from "solid-js";
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

interface Hello {
  center_hz: number;
  samplerate: number;
  fft_len: number;
  fft_rate_hz: number;
}

interface Viewport {
  startHz: number;
  stopHz: number;
}

const fullViewport = (h: Hello): Viewport => ({
  startHz: h.center_hz - h.samplerate / 2,
  stopHz: h.center_hz + h.samplerate / 2,
});

const clampViewport = (vp: Viewport, h: Hello): Viewport => {
  const lo = h.center_hz - h.samplerate / 2;
  const hi = h.center_hz + h.samplerate / 2;
  let start = Math.max(lo, Math.min(hi, vp.startHz));
  let stop = Math.max(lo, Math.min(hi, vp.stopHz));
  if (stop - start < 1) return fullViewport(h);
  return { startHz: start, stopHz: stop };
};

const SpectrumView: Component<Props> = (props) => {
  const [hello, setHello] = createSignal<Hello | null>(null);
  const [viewport, setViewport] = createSignal<Viewport | null>(null);
  const [frames, setFrames] = createSignal(0);
  const [status, setStatus] = createSignal("connecting…");
  const [audioStatus, setAudioStatus] = createSignal("idle");
  const [cmap, setCmap] = createSignal<ColormapName>(DEFAULT_COLORMAP);
  const [selection, setSelection] = createSignal<{ x0: number; x1: number } | null>(null);
  let canvas: HTMLCanvasElement | undefined;
  let audioEl: HTMLAudioElement | undefined;
  let ws: WebSocket | undefined;
  let waterfall: Waterfall | undefined;
  let pc: RTCPeerConnection | undefined;
  let history: Viewport[] = [];
  let dragOrigin: { clientX: number; rectLeft: number; rectWidth: number } | null = null;

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
    dragOrigin = {
      clientX: e.clientX,
      rectLeft: rect.left,
      rectWidth: rect.width,
    };
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
    // Treat tiny drags as a click — no zoom.
    if (xMax - xMin < 4) return;
    const span = vp.stopHz - vp.startHz;
    applyViewport(
      {
        startHz: vp.startHz + (xMin / origin.rectWidth) * span,
        stopHz: vp.startHz + (xMax / origin.rectWidth) * span,
      },
      true,
    );
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
    setAudioStatus("negotiating");
    pc = new RTCPeerConnection({
      iceServers: [{ urls: "stun:stun.l.google.com:19302" }],
    });

    pc.ontrack = (e) => {
      if (audioEl && e.streams[0]) {
        audioEl.srcObject = e.streams[0];
        audioEl.play().catch((err) => {
          setAudioStatus(`autoplay blocked: click ▶ (${err.message})`);
        });
      }
    };
    pc.onconnectionstatechange = () => {
      if (pc) setAudioStatus(pc.connectionState);
    };
    pc.onicecandidate = (e) => {
      if (e.candidate) {
        send({ type: "IceCandidate", payload: e.candidate.toJSON() });
      }
    };

    // Unreliable/unordered data channel for spectrum frames.
    const dc = pc.createDataChannel("spectrum", {
      ordered: false,
      maxRetransmits: 0,
    });
    dc.binaryType = "arraybuffer";
    dc.onmessage = (e) => {
      setFrames((n) => n + 1);
      drawFrame(new DataView(e.data as ArrayBuffer));
    };

    pc.addTransceiver("audio", { direction: "recvonly" });
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    send({ type: "Offer", payload: offer });
  };

  onMount(() => {
    if (canvas) waterfall = new Waterfall(canvas, colormap(cmap()));

    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/api/ws?token=${encodeURIComponent(props.token)}`;
    ws = new WebSocket(url);

    ws.onopen = () => setStatus("connected");
    ws.onclose = () => setStatus("disconnected");
    ws.onerror = () => setStatus("error");

    ws.onmessage = async (e) => {
      const msg = JSON.parse(e.data);
      switch (msg.type) {
        case "Hello": {
          const h = msg.payload as Hello;
          setHello(h);
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
      pc?.close();
      ws?.close();
    });
  });

  return (
    <div class="spectrum-view">
      <div class="topbar">
        <span>status: {status()}</span>
        <span>audio: {audioStatus()}</span>
        <span>frames: {frames()}</span>
        {hello() && (
          <span>
            {(hello()!.center_hz / 1e6).toFixed(3)} MHz ·{" "}
            {(hello()!.samplerate / 1000).toFixed(0)} kHz ·{" "}
            {hello()!.fft_len} bins @ {hello()!.fft_rate_hz} Hz
          </span>
        )}
        <button onClick={props.onLogout}>logout</button>
      </div>

      <div class="waterfall-wrap">
        <canvas
          ref={canvas}
          height={1024}
          class="waterfall"
          onMouseDown={onMouseDown}
          onContextMenu={onContextMenu}
          onDblClick={onDoubleClick}
        />
        {selection() && (
          <div
            class="selection-rect"
            style={{
              left: `${Math.min(selection()!.x0, selection()!.x1)}px`,
              width: `${Math.abs(selection()!.x1 - selection()!.x0)}px`,
            }}
          />
        )}
        {viewport() && (
          <div class="viewport-overlay">
            {(viewport()!.startHz / 1e6).toFixed(3)} –{" "}
            {(viewport()!.stopHz / 1e6).toFixed(3)} MHz · span{" "}
            {((viewport()!.stopHz - viewport()!.startHz) / 1e3).toFixed(1)} kHz
            {history.length > 0 && ` · ${history.length} back`}
          </div>
        )}
      </div>

      <audio ref={audioEl} controls autoplay />

      <div class="controls">
        <fieldset>
          <legend>tuning</legend>
          {hello() && (
            <FrequencyTuner
              hz={hello()!.center_hz}
              onTune={(hz) => {
                // Optimistic local update; server echoes CenterChanged.
                setHello({ ...hello()!, center_hz: hz });
                send({ type: "SetCenter", payload: { hz } });
              }}
            />
          )}
        </fieldset>
        <fieldset>
          <legend>demod</legend>
          <select disabled>
            <option>FM</option>
          </select>
        </fieldset>
        <fieldset>
          <legend>display</legend>
          <label>
            colormap
            <select
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
          </label>
        </fieldset>
      </div>
    </div>
  );
};

export default SpectrumView;
