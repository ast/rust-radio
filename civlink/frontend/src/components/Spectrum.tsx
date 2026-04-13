import { type Component, createEffect, onMount, For, Show } from "solid-js";
import type { ScopeFreq, ScopeMode } from "../types/radio";

interface SpectrumProps {
  bins: number[];
  scopeFreq?: ScopeFreq;
  frequency?: number;
  mode?: string;
  filter?: number;
  scopeMode: ScopeMode;
  fixedEdge: number;
  onClickFrequency?: (hz: number) => void;
  onSpanChange?: (spanHz: number) => void;
  onScopeModeChange?: (mode: ScopeMode) => void;
  onFixedEdgeChange?: (edge: number) => void;
}

// IC-705 valid scope spans (center mode)
const SPANS: { label: string; hz: number }[] = [
  { label: "±2.5k",  hz:    2_500 },
  { label: "±5k",    hz:    5_000 },
  { label: "±10k",   hz:   10_000 },
  { label: "±25k",   hz:   25_000 },
  { label: "±50k",   hz:   50_000 },
  { label: "±100k",  hz:  100_000 },
  { label: "±250k",  hz:  250_000 },
  { label: "±500k",  hz:  500_000 },
];

const EDGES = [1, 2, 3];

const CANVAS_WIDTH = 800;
const SPECTRUM_HEIGHT = 100;
const WATERFALL_HEIGHT = 300;
const BG_COLOR = "#0a0a0a";
const LINE_COLOR = "#4dabf5";
const FILL_COLOR = "rgba(77, 171, 245, 0.1)";
// IC-705 scope bins range 0-160
const BIN_MAX = 160;

// WebSDR-style color map: dark blue -> cyan -> green -> yellow -> red -> white
function binToColor(value: number): [number, number, number] {
  const t = Math.min(value / BIN_MAX, 1.0);

  if (t < 0.2) {
    const s = t / 0.2;
    return [0, 0, Math.round(s * 80)];
  } else if (t < 0.35) {
    const s = (t - 0.2) / 0.15;
    return [0, 0, Math.round(80 + s * 175)];
  } else if (t < 0.5) {
    const s = (t - 0.35) / 0.15;
    return [0, Math.round(s * 255), 255];
  } else if (t < 0.65) {
    const s = (t - 0.5) / 0.15;
    return [0, 255, Math.round(255 * (1 - s))];
  } else if (t < 0.8) {
    const s = (t - 0.65) / 0.15;
    return [Math.round(s * 255), 255, 0];
  } else if (t < 0.9) {
    const s = (t - 0.8) / 0.1;
    return [255, Math.round(255 * (1 - s)), 0];
  } else {
    const s = (t - 0.9) / 0.1;
    return [255, Math.round(s * 255), Math.round(s * 255)];
  }
}

// IC-705 factory default IF filter widths (Hz) per mode and filter preset.
const FILTER_WIDTHS: Record<string, [number, number, number]> = {
  Lsb:     [3000, 2400, 500],
  Usb:     [3000, 2400, 500],
  Am:      [9000, 6000, 3000],
  Cw:      [500,  250,  100],
  Fm:      [15000, 10000, 7000],
  Rtty:    [500,  250,  100],
  CwR:     [500,  250,  100],
  RttyR:   [500,  250,  100],
  DataLsb: [3000, 2400, 500],
  DataUsb: [3000, 2400, 500],
  DataFm:  [15000, 10000, 7000],
};

function getFilterWidth(mode?: string, filter?: number): number {
  if (!mode) return 3000;
  const widths = FILTER_WIDTHS[mode];
  if (!widths) return 3000;
  const idx = Math.max(0, Math.min(2, (filter ?? 1) - 1));
  return widths[idx];
}

/** Passband extent relative to the VFO marker for each demodulation mode.
 *  Returns [lowOffset, highOffset] in Hz (negative = below marker). */
function passbandOffsets(mode: string | undefined, filterHz: number): [number, number] {
  switch (mode) {
    case "Usb":
    case "DataUsb":
      return [0, filterHz];
    case "Lsb":
    case "DataLsb":
      return [-filterHz, 0];
    default:
      // CW/RTTY/FM/AM — symmetric around VFO
      return [-filterHz / 2, filterHz / 2];
  }
}

function scopeRange(f: ScopeFreq): { leftHz: number; rightHz: number } {
  if (f.mode === "center") {
    return { leftHz: f.center_hz - f.span_hz, rightHz: f.center_hz + f.span_hz };
  }
  return { leftHz: f.lower_hz, rightHz: f.upper_hz };
}

const MARKER_COLOR = "rgba(255, 50, 50, 0.9)";
const PASSBAND_COLOR = "rgba(255, 255, 255, 0.06)";
const PASSBAND_EDGE_COLOR = "rgba(255, 255, 255, 0.15)";

const Spectrum: Component<SpectrumProps> = (props) => {
  let specCanvas: HTMLCanvasElement | undefined;
  let wfCanvas: HTMLCanvasElement | undefined;

  const handleCanvasClick = (e: MouseEvent) => {
    const sf = props.scopeFreq;
    if (!sf || !props.onClickFrequency) return;
    const { leftHz, rightHz } = scopeRange(sf);
    const canvas = e.currentTarget as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const freq = leftHz + x * (rightHz - leftHz);
    props.onClickFrequency(Math.round(freq));
  };

  onMount(() => {
    if (specCanvas) {
      specCanvas.width = CANVAS_WIDTH;
      specCanvas.height = SPECTRUM_HEIGHT;
      const ctx = specCanvas.getContext("2d")!;
      ctx.fillStyle = BG_COLOR;
      ctx.fillRect(0, 0, CANVAS_WIDTH, SPECTRUM_HEIGHT);
    }
    if (wfCanvas) {
      wfCanvas.width = CANVAS_WIDTH;
      wfCanvas.height = WATERFALL_HEIGHT;
      const ctx = wfCanvas.getContext("2d")!;
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, CANVAS_WIDTH, WATERFALL_HEIGHT);
    }
  });

  createEffect(() => {
    const bins = props.bins;
    if (bins.length === 0) return;

    // --- Spectrum line ---
    if (specCanvas) {
      const ctx = specCanvas.getContext("2d")!;
      ctx.fillStyle = BG_COLOR;
      ctx.fillRect(0, 0, CANVAS_WIDTH, SPECTRUM_HEIGHT);

      const n = bins.length;
      const xStep = CANVAS_WIDTH / n;

      ctx.beginPath();
      ctx.moveTo(0, SPECTRUM_HEIGHT);
      for (let i = 0; i < n; i++) {
        const y = SPECTRUM_HEIGHT - (bins[i] / BIN_MAX) * SPECTRUM_HEIGHT;
        ctx.lineTo(i * xStep, y);
      }
      ctx.lineTo(CANVAS_WIDTH, SPECTRUM_HEIGHT);
      ctx.closePath();
      ctx.fillStyle = FILL_COLOR;
      ctx.fill();

      ctx.beginPath();
      for (let i = 0; i < n; i++) {
        const y = SPECTRUM_HEIGHT - (bins[i] / BIN_MAX) * SPECTRUM_HEIGHT;
        const x = i * xStep;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = LINE_COLOR;
      ctx.lineWidth = 1;
      ctx.stroke();

      // --- Tuning indicator + filter passband ---
      const sf = props.scopeFreq;
      const freq = props.frequency;
      if (sf && freq) {
        const { leftHz, rightHz } = scopeRange(sf);
        const widthHz = rightHz - leftHz;
        if (widthHz > 0 && freq >= leftHz && freq <= rightHz) {
          const hzToX = (hz: number) => ((hz - leftHz) / widthHz) * CANVAS_WIDTH;
          const cx = hzToX(freq);
          const filterHz = getFilterWidth(props.mode, props.filter);
          const [loOff, hiOff] = passbandOffsets(props.mode, filterHz);
          const x0 = hzToX(freq + loOff);
          const x1 = hzToX(freq + hiOff);

          ctx.fillStyle = PASSBAND_COLOR;
          ctx.fillRect(x0, 0, x1 - x0, SPECTRUM_HEIGHT);

          ctx.strokeStyle = PASSBAND_EDGE_COLOR;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(x0, 0); ctx.lineTo(x0, SPECTRUM_HEIGHT);
          ctx.moveTo(x1, 0); ctx.lineTo(x1, SPECTRUM_HEIGHT);
          ctx.stroke();

          ctx.strokeStyle = MARKER_COLOR;
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(cx, 0);
          ctx.lineTo(cx, SPECTRUM_HEIGHT);
          ctx.stroke();
        }
      }
    }

    // --- Waterfall ---
    if (wfCanvas) {
      const ctx = wfCanvas.getContext("2d")!;
      const n = bins.length;

      ctx.drawImage(wfCanvas, 0, 0, CANVAS_WIDTH, WATERFALL_HEIGHT, 0, 1, CANVAS_WIDTH, WATERFALL_HEIGHT);

      const row = ctx.createImageData(CANVAS_WIDTH, 1);
      const xStep = n / CANVAS_WIDTH;
      for (let x = 0; x < CANVAS_WIDTH; x++) {
        const binIdx = Math.min(Math.floor(x * xStep), n - 1);
        const [r, g, b] = binToColor(bins[binIdx]);
        const off = x * 4;
        row.data[off] = r;
        row.data[off + 1] = g;
        row.data[off + 2] = b;
        row.data[off + 3] = 255;
      }
      ctx.putImageData(row, 0, 0);
    }
  });

  const currentSpan = () => {
    const sf = props.scopeFreq;
    if (sf && sf.mode === "center") return sf.span_hz;
    return 0;
  };

  return (
    <>
      <div class="span-selector">
        <span class="ctrl-label">Scope</span>
        <button
          class={`span-btn ${props.scopeMode === "center" ? "span-btn-active" : ""}`}
          onClick={() => props.onScopeModeChange?.("center")}
        >
          Center
        </button>
        <button
          class={`span-btn ${props.scopeMode === "fixed" ? "span-btn-active" : ""}`}
          onClick={() => props.onScopeModeChange?.("fixed")}
        >
          Fixed
        </button>
        <Show when={props.scopeMode === "center"}>
          <span class="ctrl-label">Span</span>
          <For each={SPANS}>{(s) =>
            <button
              class={`span-btn ${currentSpan() === s.hz ? "span-btn-active" : ""}`}
              onClick={() => props.onSpanChange?.(s.hz)}
            >
              {s.label}
            </button>
          }</For>
        </Show>
        <Show when={props.scopeMode === "fixed"}>
          <span class="ctrl-label">Edge</span>
          <For each={EDGES}>{(n) =>
            <button
              class={`span-btn ${props.fixedEdge === n ? "span-btn-active" : ""}`}
              onClick={() => props.onFixedEdgeChange?.(n)}
            >
              {n}
            </button>
          }</For>
        </Show>
      </div>
      <canvas ref={specCanvas} width={CANVAS_WIDTH} height={SPECTRUM_HEIGHT} class="spectrum-clickable" onClick={handleCanvasClick} />
      <canvas ref={wfCanvas} width={CANVAS_WIDTH} height={WATERFALL_HEIGHT} class="spectrum-clickable" style={{ display: "block" }} onClick={handleCanvasClick} />
    </>
  );
};

export default Spectrum;
