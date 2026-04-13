import { type Component, createEffect, onMount } from "solid-js";
import type { StatsSample } from "../api/stats";

interface StatsGraphProps {
  history: StatsSample[];
  height?: number;
}

const COLOR_IN = "#4dabf5";
const COLOR_OUT = "#ffa726";
const COLOR_GRID = "#2e2e2e";
const COLOR_TEXT = "#777777";

const StatsGraph: Component<StatsGraphProps> = (props) => {
  let canvas: HTMLCanvasElement | undefined;

  const draw = () => {
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const cssW = canvas.clientWidth;
    const cssH = props.height ?? 64;
    if (canvas.width !== cssW * dpr || canvas.height !== cssH * dpr) {
      canvas.width = cssW * dpr;
      canvas.height = cssH * dpr;
      canvas.style.height = `${cssH}px`;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);

    const samples = props.history;
    if (samples.length < 2) {
      ctx.fillStyle = COLOR_TEXT;
      ctx.font = "10px 'Share Tech Mono', monospace";
      ctx.fillText("collecting…", 4, 12);
      return;
    }

    let max = 1;
    for (const s of samples) {
      if (s.kbpsIn > max) max = s.kbpsIn;
      if (s.kbpsOut > max) max = s.kbpsOut;
    }
    max = Math.ceil(max * 1.2);

    ctx.strokeStyle = COLOR_GRID;
    ctx.lineWidth = 1;
    for (let i = 1; i < 4; i++) {
      const y = Math.round((cssH * i) / 4) + 0.5;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(cssW, y);
      ctx.stroke();
    }

    const n = samples.length;
    const stepX = cssW / Math.max(1, n - 1);
    const plot = (key: "kbpsIn" | "kbpsOut", color: string) => {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.25;
      ctx.beginPath();
      samples.forEach((s, i) => {
        const x = i * stepX;
        const y = cssH - (s[key] / max) * (cssH - 2) - 1;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    };
    plot("kbpsIn", COLOR_IN);
    plot("kbpsOut", COLOR_OUT);

    ctx.fillStyle = COLOR_TEXT;
    ctx.font = "10px 'Share Tech Mono', monospace";
    ctx.fillText(`${max} kb/s`, 4, 10);
  };

  onMount(draw);
  createEffect(() => {
    props.history;
    draw();
  });

  return (
    <div class="stats-graph">
      <div class="stats-graph-legend">
        <span class="legend-in">■ rx</span>
        <span class="legend-out">■ tx</span>
      </div>
      <canvas ref={canvas} />
    </div>
  );
};

export default StatsGraph;
