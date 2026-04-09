import { type Component, createEffect, onMount } from "solid-js";

interface SpectrumProps {
  bins: number[];
}

const CANVAS_WIDTH = 800;
const CANVAS_HEIGHT = 200;
const BG_COLOR = "#0f0f23";
const LINE_COLOR = "#00d4ff";
const FILL_COLOR = "rgba(0, 212, 255, 0.15)";
// IC-705 scope bins range 0-160
const BIN_MAX = 160;

const Spectrum: Component<SpectrumProps> = (props) => {
  let canvas: HTMLCanvasElement | undefined;

  onMount(() => {
    if (!canvas) return;
    // Set backing store to match CSS size
    canvas.width = CANVAS_WIDTH;
    canvas.height = CANVAS_HEIGHT;
    // Draw empty background
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = BG_COLOR;
    ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
  });

  createEffect(() => {
    const bins = props.bins;
    if (!canvas || bins.length === 0) return;

    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = BG_COLOR;
    ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    const n = bins.length;
    const xStep = CANVAS_WIDTH / n;

    ctx.beginPath();
    ctx.moveTo(0, CANVAS_HEIGHT);

    for (let i = 0; i < n; i++) {
      const amplitude = bins[i] / BIN_MAX;
      const y = CANVAS_HEIGHT - amplitude * CANVAS_HEIGHT;
      const x = i * xStep;
      ctx.lineTo(x, y);
    }

    // Close the filled area
    ctx.lineTo(CANVAS_WIDTH, CANVAS_HEIGHT);
    ctx.closePath();
    ctx.fillStyle = FILL_COLOR;
    ctx.fill();

    // Draw the line on top
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
      const amplitude = bins[i] / BIN_MAX;
      const y = CANVAS_HEIGHT - amplitude * CANVAS_HEIGHT;
      const x = i * xStep;
      if (i === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    ctx.strokeStyle = LINE_COLOR;
    ctx.lineWidth = 1;
    ctx.stroke();
  });

  return (
    <div class="spectrum">
      <canvas ref={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT} />
    </div>
  );
};

export default Spectrum;
