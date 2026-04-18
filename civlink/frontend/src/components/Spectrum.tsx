import { type Component, createEffect, createSignal, onMount, onCleanup, For, Show } from "solid-js";
import type { ScopeFreq, ScopeMode } from "../types/radio";
import { theme } from "../theme";

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

const BIN_MAX = 160;

// CSS-pixel heights; canvas physical pixels are these × dpr.
const DESKTOP_SPEC_H = 100;
const DESKTOP_WF_H = 300;
const MOBILE_SPEC_H = 80;
const MOBILE_WF_H = 180;

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

/** Passband extent relative to the VFO marker for each demodulation mode. */
function passbandOffsets(mode: string | undefined, filterHz: number): [number, number] {
  switch (mode) {
    case "Usb":
    case "DataUsb":
      return [0, filterHz];
    case "Lsb":
    case "DataLsb":
      return [-filterHz, 0];
    default:
      return [-filterHz / 2, filterHz / 2];
  }
}

function scopeRange(f: ScopeFreq): { leftHz: number; rightHz: number } {
  if (f.mode === "center") {
    return { leftHz: f.center_hz - f.span_hz, rightHz: f.center_hz + f.span_hz };
  }
  return { leftHz: f.lower_hz, rightHz: f.upper_hz };
}

interface CanvasSize {
  cssW: number;
  specH: number;
  wfH: number;
  dpr: number;
}

const Spectrum: Component<SpectrumProps> = (props) => {
  let container: HTMLDivElement | undefined;
  let specCanvas: HTMLCanvasElement | undefined;
  let wfCanvas: HTMLCanvasElement | undefined;

  const [size, setSize] = createSignal<CanvasSize>({ cssW: 800, specH: DESKTOP_SPEC_H, wfH: DESKTOP_WF_H, dpr: 1 });

  const resize = () => {
    if (!container) return;
    const cssW = Math.max(1, Math.floor(container.clientWidth));
    const narrow = window.innerWidth < 600;
    const dpr = window.devicePixelRatio || 1;
    const specH = narrow ? MOBILE_SPEC_H : DESKTOP_SPEC_H;
    const wfH = narrow ? MOBILE_WF_H : DESKTOP_WF_H;

    if (specCanvas) {
      specCanvas.width = cssW * dpr;
      specCanvas.height = specH * dpr;
      specCanvas.style.height = `${specH}px`;
    }
    if (wfCanvas) {
      wfCanvas.width = cssW * dpr;
      wfCanvas.height = wfH * dpr;
      wfCanvas.style.height = `${wfH}px`;
      const ctx = wfCanvas.getContext("2d")!;
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, wfCanvas.width, wfCanvas.height);
    }
    setSize({ cssW, specH, wfH, dpr });
  };

  onMount(() => {
    resize();
    const obs = new ResizeObserver(resize);
    if (container) obs.observe(container);
    window.addEventListener("resize", resize);
    onCleanup(() => {
      obs.disconnect();
      window.removeEventListener("resize", resize);
    });
  });

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

  const handleCanvasWheel = (e: WheelEvent) => {
    if (!props.onClickFrequency || props.frequency === undefined) return;
    e.preventDefault();
    const step = e.shiftKey ? 1_000 : 100;
    const delta = e.deltaY < 0 ? step : -step;
    props.onClickFrequency(props.frequency + delta);
  };

  createEffect(() => {
    const bins = props.bins;
    const { cssW, specH, wfH, dpr } = size();
    if (bins.length === 0) return;

    if (specCanvas) {
      const ctx = specCanvas.getContext("2d")!;
      const W = cssW * dpr;
      const H = specH * dpr;
      ctx.fillStyle = theme.spectrum.bg;
      ctx.fillRect(0, 0, W, H);

      const n = bins.length;
      const xStep = W / n;

      ctx.beginPath();
      ctx.moveTo(0, H);
      for (let i = 0; i < n; i++) {
        const y = H - (bins[i] / BIN_MAX) * H;
        ctx.lineTo(i * xStep, y);
      }
      ctx.lineTo(W, H);
      ctx.closePath();
      ctx.fillStyle = theme.spectrum.fill;
      ctx.fill();

      ctx.beginPath();
      for (let i = 0; i < n; i++) {
        const y = H - (bins[i] / BIN_MAX) * H;
        const x = i * xStep;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = theme.spectrum.line;
      ctx.lineWidth = dpr;
      ctx.stroke();

      const sf = props.scopeFreq;
      const freq = props.frequency;
      if (sf && freq) {
        const { leftHz, rightHz } = scopeRange(sf);
        const widthHz = rightHz - leftHz;
        if (widthHz > 0 && freq >= leftHz && freq <= rightHz) {
          const hzToX = (hz: number) => ((hz - leftHz) / widthHz) * W;
          const cx = hzToX(freq);
          const filterHz = getFilterWidth(props.mode, props.filter);
          const [loOff, hiOff] = passbandOffsets(props.mode, filterHz);
          const x0 = hzToX(freq + loOff);
          const x1 = hzToX(freq + hiOff);

          ctx.fillStyle = theme.spectrum.passband;
          ctx.fillRect(x0, 0, x1 - x0, H);

          ctx.strokeStyle = theme.spectrum.passbandEdge;
          ctx.lineWidth = dpr;
          ctx.beginPath();
          ctx.moveTo(x0, 0); ctx.lineTo(x0, H);
          ctx.moveTo(x1, 0); ctx.lineTo(x1, H);
          ctx.stroke();

          ctx.strokeStyle = theme.spectrum.marker;
          ctx.lineWidth = dpr;
          ctx.beginPath();
          ctx.moveTo(cx, 0);
          ctx.lineTo(cx, H);
          ctx.stroke();
        }
      }
    }

    if (wfCanvas) {
      const ctx = wfCanvas.getContext("2d")!;
      const W = cssW * dpr;
      const H = wfH * dpr;
      const rowH = Math.max(1, Math.round(dpr));
      const n = bins.length;

      // Scroll existing content down by rowH physical pixels.
      ctx.drawImage(wfCanvas, 0, 0, W, H, 0, rowH, W, H);

      const row = ctx.createImageData(W, rowH);
      const xStep = n / W;
      for (let x = 0; x < W; x++) {
        const binIdx = Math.min(Math.floor(x * xStep), n - 1);
        const [r, g, b] = theme.waterfall(bins[binIdx] / BIN_MAX);
        for (let yy = 0; yy < rowH; yy++) {
          const off = (yy * W + x) * 4;
          row.data[off] = r;
          row.data[off + 1] = g;
          row.data[off + 2] = b;
          row.data[off + 3] = 255;
        }
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
      <div ref={container}>
        <canvas ref={specCanvas} class="spectrum-clickable" onClick={handleCanvasClick} onWheel={handleCanvasWheel} />
        <canvas ref={wfCanvas} class="spectrum-clickable" onClick={handleCanvasClick} onWheel={handleCanvasWheel} />
      </div>
    </>
  );
};

export default Spectrum;
