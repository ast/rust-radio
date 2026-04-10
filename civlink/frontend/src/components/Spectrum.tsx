import { type Component, createEffect, onMount } from "solid-js";

interface SpectrumProps {
  bins: number[];
}

const CANVAS_WIDTH = 800;
const SPECTRUM_HEIGHT = 100;
const WATERFALL_HEIGHT = 300;
const BG_COLOR = "#0f0f23";
const LINE_COLOR = "#00d4ff";
const FILL_COLOR = "rgba(0, 212, 255, 0.15)";
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

const Spectrum: Component<SpectrumProps> = (props) => {
  let specCanvas: HTMLCanvasElement | undefined;
  let wfCanvas: HTMLCanvasElement | undefined;

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
    <div class="spectrum">
      <canvas ref={specCanvas} width={CANVAS_WIDTH} height={SPECTRUM_HEIGHT} />
      <canvas ref={wfCanvas} width={CANVAS_WIDTH} height={WATERFALL_HEIGHT} style={{ display: "block" }} />
    </div>
  );
};

export default Spectrum;
