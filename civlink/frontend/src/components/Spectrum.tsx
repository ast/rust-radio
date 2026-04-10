import { type Component, createEffect, onMount } from "solid-js";

interface SpectrumProps {
  bins: number[];
  centerHz?: number;
  spanHz?: number;
  frequency?: number;
  mode?: string;
  filter?: number;
  onClickFrequency?: (hz: number) => void;
}

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
    // Black -> dark blue
    const s = t / 0.2;
    return [0, 0, Math.round(s * 80)];
  } else if (t < 0.35) {
    // Dark blue -> blue
    const s = (t - 0.2) / 0.15;
    return [0, 0, Math.round(80 + s * 175)];
  } else if (t < 0.5) {
    // Blue -> cyan
    const s = (t - 0.35) / 0.15;
    return [0, Math.round(s * 255), 255];
  } else if (t < 0.65) {
    // Cyan -> green
    const s = (t - 0.5) / 0.15;
    return [0, 255, Math.round(255 * (1 - s))];
  } else if (t < 0.8) {
    // Green -> yellow
    const s = (t - 0.65) / 0.15;
    return [Math.round(s * 255), 255, 0];
  } else if (t < 0.9) {
    // Yellow -> red
    const s = (t - 0.8) / 0.1;
    return [255, Math.round(255 * (1 - s)), 0];
  } else {
    // Red -> white
    const s = (t - 0.9) / 0.1;
    return [255, Math.round(s * 255), Math.round(s * 255)];
  }
}

// IC-705 factory default IF filter widths (Hz) per mode and filter preset.
// FIL1 is widest, FIL3 is narrowest. These are approximate factory defaults.
const FILTER_WIDTHS: Record<string, [number, number, number]> = {
  // [FIL1, FIL2, FIL3]
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

/** Get filter width in Hz for current mode and filter preset (1-3) */
function getFilterWidth(mode?: string, filter?: number): number {
  if (!mode) return 3000;
  const widths = FILTER_WIDTHS[mode];
  if (!widths) return 3000;
  const idx = Math.max(0, Math.min(2, (filter ?? 1) - 1));
  return widths[idx];
}

/** Convert a frequency to a canvas X pixel position */
function freqToX(freq: number, centerHz: number, spanHz: number): number {
  return ((freq - centerHz + spanHz / 2) / spanHz) * CANVAS_WIDTH;
}

const MARKER_COLOR = "rgba(255, 50, 50, 0.9)";
const PASSBAND_COLOR = "rgba(255, 255, 255, 0.06)";
const PASSBAND_EDGE_COLOR = "rgba(255, 255, 255, 0.15)";

const Spectrum: Component<SpectrumProps> = (props) => {
  let specCanvas: HTMLCanvasElement | undefined;
  let wfCanvas: HTMLCanvasElement | undefined;

  const handleCanvasClick = (e: MouseEvent) => {
    const center = props.centerHz;
    const span = props.spanHz;
    if (!center || !span || !props.onClickFrequency) return;
    const canvas = e.currentTarget as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    const x = (e.clientX - rect.left) / rect.width;
    const freq = center - span / 2 + x * span;
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

      // Filled area
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

      // Line trace
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
      const center = props.centerHz;
      const span = props.spanHz;
      const freq = props.frequency;
      if (center && span && freq) {
        const cx = freqToX(freq, center, span);
        const filterHz = getFilterWidth(props.mode, props.filter);
        const halfW = (filterHz / span) * CANVAS_WIDTH / 2;

        // Passband fill
        ctx.fillStyle = PASSBAND_COLOR;
        ctx.fillRect(cx - halfW, 0, halfW * 2, SPECTRUM_HEIGHT);

        // Passband edges
        ctx.strokeStyle = PASSBAND_EDGE_COLOR;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(cx - halfW, 0);
        ctx.lineTo(cx - halfW, SPECTRUM_HEIGHT);
        ctx.moveTo(cx + halfW, 0);
        ctx.lineTo(cx + halfW, SPECTRUM_HEIGHT);
        ctx.stroke();

        // Center line
        ctx.strokeStyle = MARKER_COLOR;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(cx, 0);
        ctx.lineTo(cx, SPECTRUM_HEIGHT);
        ctx.stroke();
      }
    }

    // --- Waterfall ---
    if (wfCanvas) {
      const ctx = wfCanvas.getContext("2d")!;
      const n = bins.length;

      // Scroll existing content down by 1 pixel
      ctx.drawImage(wfCanvas, 0, 0, CANVAS_WIDTH, WATERFALL_HEIGHT, 0, 1, CANVAS_WIDTH, WATERFALL_HEIGHT);

      // Draw new row at top
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

  return (
    <>
      <canvas ref={specCanvas} width={CANVAS_WIDTH} height={SPECTRUM_HEIGHT} class="spectrum-clickable" onClick={handleCanvasClick} />
      <canvas ref={wfCanvas} width={CANVAS_WIDTH} height={WATERFALL_HEIGHT} class="spectrum-clickable" style={{ display: "block" }} onClick={handleCanvasClick} />
    </>
  );
};

export default Spectrum;
