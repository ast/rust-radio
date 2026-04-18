import type { Theme } from "../types";
import { websdr } from "../colormaps";

const ACCENT = "#4dabf5";
const ORANGE = "#ffa726";

const rsba1Dark: Theme = {
  id: "rsba1-dark",
  tokens: {
    bg: "#1c1c1c",
    surface: "#222222",
    surfaceInset: "#181818",
    border: "#2e2e2e",
    borderStrong: "#3a3a3a",

    text: "#cccccc",
    textDim: "#777777",
    textBright: "#ffffff",

    action: ACCENT,
    actionDim: "rgba(77, 171, 245, 0.15)",
    focus: ACCENT,
    selected: ACCENT,
    selectedDim: "rgba(77, 171, 245, 0.15)",
    selectedBorder: "rgba(77, 171, 245, 0.25)",

    danger: "#ef5350",
    warning: ORANGE,
    warningDim: "rgba(255, 167, 38, 0.15)",
    warningBorder: "rgba(255, 167, 38, 0.25)",
    success: "#4caf50",

    rx: ACCENT,
    tx: ORANGE,
    mhz: ACCENT,
    khz: "#ffffff",
    hz: "#777777",
  },
  spectrum: {
    bg: "#0a0a0a",
    line: ACCENT,
    fill: "rgba(77, 171, 245, 0.1)",
    marker: "rgba(255, 50, 50, 0.9)",
    passband: "rgba(255, 255, 255, 0.06)",
    passbandEdge: "rgba(255, 255, 255, 0.15)",
  },
  stats: {
    rx: ACCENT,
    tx: ORANGE,
    grid: "#2e2e2e",
    text: "#777777",
    bg: "#181818",
  },
  waterfall: websdr,
};

export default rsba1Dark;
