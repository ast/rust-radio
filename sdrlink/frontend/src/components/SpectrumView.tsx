import { Component, createSignal, onCleanup, onMount } from "solid-js";
import {
  COLORMAP_NAMES,
  ColormapName,
  DEFAULT_COLORMAP,
  colormap,
} from "../colormap";
import { Waterfall } from "../waterfall";

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

const SpectrumView: Component<Props> = (props) => {
  const [hello, setHello] = createSignal<Hello | null>(null);
  const [frames, setFrames] = createSignal(0);
  const [status, setStatus] = createSignal("connecting…");
  const [audioStatus, setAudioStatus] = createSignal("idle");
  const [cmap, setCmap] = createSignal<ColormapName>(DEFAULT_COLORMAP);
  let canvas: HTMLCanvasElement | undefined;
  let audioEl: HTMLAudioElement | undefined;
  let ws: WebSocket | undefined;
  let waterfall: Waterfall | undefined;
  let pc: RTCPeerConnection | undefined;

  const send = (msg: unknown) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(msg));
  };

  const sendViewport = (h: Hello) => {
    if (!canvas) return;
    const pixels = Math.max(64, Math.min(canvas.clientWidth || 1024, h.fft_len));
    const halfBand = h.samplerate / 2;
    send({
      type: "SetViewport",
      payload: {
        start_hz: h.center_hz - halfBand,
        stop_hz: h.center_hz + halfBand,
        pixels,
      },
    });
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
    ws.binaryType = "arraybuffer";

    ws.onopen = () => setStatus("connected");
    ws.onclose = () => setStatus("disconnected");
    ws.onerror = () => setStatus("error");

    ws.onmessage = async (e) => {
      if (typeof e.data === "string") {
        const msg = JSON.parse(e.data);
        switch (msg.type) {
          case "Hello": {
            const h = msg.payload as Hello;
            setHello(h);
            sendViewport(h);
            await startWebrtc();
            break;
          }
          case "Answer":
            if (pc) await pc.setRemoteDescription(msg.payload);
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
        return;
      }
      setFrames((n) => n + 1);
      drawFrame(new DataView(e.data));
    };

    const drawFrame = (view: DataView) => {
      if (!waterfall) return;
      const pixels = view.getUint16(0, true);
      if (pixels === 0) return;
      const max = new Uint8Array(view.buffer, 2 + pixels, pixels);
      waterfall.pushRow(max);
    };

    const resizeObserver = new ResizeObserver(() => {
      const h = hello();
      if (h) sendViewport(h);
    });
    if (canvas) resizeObserver.observe(canvas);

    onCleanup(() => {
      resizeObserver.disconnect();
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

      <canvas ref={canvas} height={1024} class="waterfall" />

      <audio ref={audioEl} controls autoplay />

      <div class="controls">
        <fieldset>
          <legend>tuning</legend>
          <label>
            center (MHz)
            <input
              type="number"
              step="0.001"
              disabled
              value={hello() ? (hello()!.center_hz / 1e6).toFixed(3) : ""}
            />
          </label>
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
