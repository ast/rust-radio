import { Component, createSignal, onCleanup, onMount } from "solid-js";
import { buildColormap } from "../colormap";

interface Props {
  token: string;
  onLogout: () => void;
}

interface Hello {
  type: "Hello";
  center_hz: number;
  samplerate: number;
  fft_len: number;
  fft_rate_hz: number;
}

const COLORMAP = buildColormap();

const SpectrumView: Component<Props> = (props) => {
  const [hello, setHello] = createSignal<Hello | null>(null);
  const [frames, setFrames] = createSignal(0);
  const [status, setStatus] = createSignal("connecting…");
  let canvas: HTMLCanvasElement | undefined;
  let ws: WebSocket | undefined;

  const sendViewport = (h: Hello) => {
    if (!ws || !canvas || ws.readyState !== WebSocket.OPEN) return;
    const pixels = Math.max(
      64,
      Math.min(canvas.clientWidth || 1024, h.fft_len),
    );
    const halfBand = h.samplerate / 2;
    ws.send(
      JSON.stringify({
        type: "SetViewport",
        start_hz: h.center_hz - halfBand,
        stop_hz: h.center_hz + halfBand,
        pixels,
      }),
    );
  };

  onMount(() => {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/api/ws?token=${encodeURIComponent(props.token)}`;
    ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";

    ws.onopen = () => setStatus("connected");
    ws.onclose = () => setStatus("disconnected");
    ws.onerror = () => setStatus("error");

    ws.onmessage = (e) => {
      if (typeof e.data === "string") {
        const msg = JSON.parse(e.data);
        if (msg.type === "Hello") {
          const h = msg as Hello;
          setHello(h);
          sendViewport(h);
        }
        return;
      }
      setFrames((n) => n + 1);
      drawFrame(new DataView(e.data));
    };

    const drawFrame = (view: DataView) => {
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const pixels = view.getUint16(0, true);
      if (pixels === 0) return;
      const max = new Uint8Array(view.buffer, 2 + pixels, pixels);

      if (canvas.width !== pixels) canvas.width = pixels;
      const { width, height } = canvas;

      const img = ctx.getImageData(0, 0, width, height);
      img.data.copyWithin(0, width * 4, width * height * 4);
      const off = (height - 1) * width * 4;
      for (let i = 0; i < pixels; i++) {
        const c = max[i] * 3;
        const j = off + i * 4;
        img.data[j] = COLORMAP[c];
        img.data[j + 1] = COLORMAP[c + 1];
        img.data[j + 2] = COLORMAP[c + 2];
        img.data[j + 3] = 255;
      }
      ctx.putImageData(img, 0, 0);
    };

    // Re-send viewport on resize so server decimates to match new canvas width.
    const resizeObserver = new ResizeObserver(() => {
      const h = hello();
      if (h) sendViewport(h);
    });
    if (canvas) resizeObserver.observe(canvas);

    onCleanup(() => {
      resizeObserver.disconnect();
      ws?.close();
    });
  });

  return (
    <div class="spectrum-view">
      <div class="topbar">
        <span>status: {status()}</span>
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

      <canvas
        ref={canvas}
        height={1024}
        class="waterfall"
      />

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
            <option>FM (coming soon)</option>
          </select>
        </fieldset>
        <fieldset>
          <legend>display</legend>
          <label>
            colormap
            <select disabled>
              <option>websdr</option>
            </select>
          </label>
        </fieldset>
      </div>
    </div>
  );
};

export default SpectrumView;
