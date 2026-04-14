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
  type: "Hello";
  center_hz: number;
  samplerate: number;
  fft_len: number;
  fft_rate_hz: number;
}

const SpectrumView: Component<Props> = (props) => {
  const [hello, setHello] = createSignal<Hello | null>(null);
  const [frames, setFrames] = createSignal(0);
  const [status, setStatus] = createSignal("connecting…");
  const [cmap, setCmap] = createSignal<ColormapName>(DEFAULT_COLORMAP);
  let canvas: HTMLCanvasElement | undefined;
  let ws: WebSocket | undefined;
  let waterfall: Waterfall | undefined;

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
    if (canvas) waterfall = new Waterfall(canvas, colormap(cmap()));

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
      if (!waterfall) return;
      const pixels = view.getUint16(0, true);
      if (pixels === 0) return;
      // Layout: [u16 pixels][u8 min[pixels]][u8 max[pixels]]
      const max = new Uint8Array(view.buffer, 2 + pixels, pixels);
      waterfall.pushRow(max);
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
