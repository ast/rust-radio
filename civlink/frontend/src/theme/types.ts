/** A Theme is a pure data object: the tokens applied to :root at startup,
 *  plus the non-token colors the canvas components read directly. */
export interface Theme {
  id: string;

  /** CSS custom property values, applied to document.documentElement. */
  tokens: ThemeTokens;

  /** Colors the <canvas> components read from the theme directly rather
   *  than via getComputedStyle — drawing hot paths should not round-trip
   *  through the CSSOM. */
  spectrum: SpectrumColors;
  stats: StatsColors;

  /** Waterfall colormap: map a normalized value in [0,1] to [r,g,b] 0-255. */
  waterfall: Colormap;
}

export interface ThemeTokens {
  // surface
  bg: string;
  surface: string;
  surfaceInset: string;
  border: string;
  borderStrong: string;

  // text
  text: string;
  textDim: string;
  textBright: string;

  // roles
  action: string;
  actionDim: string;
  focus: string;
  selected: string;
  selectedDim: string;
  selectedBorder: string;

  // semantic
  danger: string;
  warning: string;
  warningDim: string;
  warningBorder: string;
  success: string;

  // data
  rx: string;
  tx: string;
  mhz: string;
  khz: string;
  hz: string;
}

export interface SpectrumColors {
  bg: string;
  line: string;
  fill: string;
  marker: string;
  passband: string;
  passbandEdge: string;
}

export interface StatsColors {
  rx: string;
  tx: string;
  grid: string;
  text: string;
  bg: string;
}

export type Colormap = (t: number) => [number, number, number];
