// WebSDR-style waterfall colormap: black → dark blue → blue → cyan → green →
// yellow → red → white. Precomputed as a flat RGB lookup (256 entries × 3
// bytes) for per-bin use in the draw loop.

type Stop = [number, [number, number, number]];

const STOPS: Stop[] = [
  [0.0, [0, 0, 0]],
  [0.15, [0, 0, 80]],
  [0.3, [0, 0, 200]],
  [0.45, [0, 200, 255]],
  [0.6, [0, 220, 0]],
  [0.75, [255, 220, 0]],
  [0.9, [255, 60, 0]],
  [1.0, [255, 255, 255]],
];

function interp(a: number, b: number, t: number): number {
  return Math.round(a + (b - a) * t);
}

export function buildColormap(): Uint8Array {
  const out = new Uint8Array(256 * 3);
  for (let i = 0; i < 256; i++) {
    const f = i / 255;
    let lo = STOPS[0];
    let hi = STOPS[STOPS.length - 1];
    for (let s = 0; s < STOPS.length - 1; s++) {
      if (f >= STOPS[s][0] && f <= STOPS[s + 1][0]) {
        lo = STOPS[s];
        hi = STOPS[s + 1];
        break;
      }
    }
    const span = hi[0] - lo[0];
    const t = span > 0 ? (f - lo[0]) / span : 0;
    out[i * 3 + 0] = interp(lo[1][0], hi[1][0], t);
    out[i * 3 + 1] = interp(lo[1][1], hi[1][1], t);
    out[i * 3 + 2] = interp(lo[1][2], hi[1][2], t);
  }
  return out;
}
