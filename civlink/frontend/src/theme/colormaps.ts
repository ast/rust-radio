import type { Colormap } from "./types";

/** WebSDR-style rainbow: dark blue → cyan → green → yellow → red → white. */
export const websdr: Colormap = (t) => {
  const x = Math.min(Math.max(t, 0), 1);
  if (x < 0.2) {
    const s = x / 0.2;
    return [0, 0, Math.round(s * 80)];
  }
  if (x < 0.35) {
    const s = (x - 0.2) / 0.15;
    return [0, 0, Math.round(80 + s * 175)];
  }
  if (x < 0.5) {
    const s = (x - 0.35) / 0.15;
    return [0, Math.round(s * 255), 255];
  }
  if (x < 0.65) {
    const s = (x - 0.5) / 0.15;
    return [0, 255, Math.round(255 * (1 - s))];
  }
  if (x < 0.8) {
    const s = (x - 0.65) / 0.15;
    return [Math.round(s * 255), 255, 0];
  }
  if (x < 0.9) {
    const s = (x - 0.8) / 0.1;
    return [255, Math.round(255 * (1 - s)), 0];
  }
  const s = (x - 0.9) / 0.1;
  return [255, Math.round(s * 255), Math.round(s * 255)];
};
