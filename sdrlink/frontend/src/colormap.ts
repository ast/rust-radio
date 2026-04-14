// Perceptually uniform colormaps (matplotlib: viridis, plasma, inferno,
// magma, cividis). Polynomial approximations from
// https://www.shadertoy.com/view/WlfXRN — good to within a few LSB across
// the range and tiny compared to shipping the full 256-entry tables.
//
// Each colormap is exposed as a 256×3 Uint8Array suitable for uploading as
// a 256×1 RGB texture.

type Vec3 = readonly [number, number, number];

function clamp01(x: number): number {
  return x < 0 ? 0 : x > 1 ? 1 : x;
}

function polyLut(coefs: readonly Vec3[]): Uint8Array {
  const out = new Uint8Array(256 * 3);
  for (let i = 0; i < 256; i++) {
    const t = i / 255;
    // Horner evaluation of the 6th-order polynomial.
    let r = coefs[coefs.length - 1][0];
    let g = coefs[coefs.length - 1][1];
    let b = coefs[coefs.length - 1][2];
    for (let k = coefs.length - 2; k >= 0; k--) {
      r = r * t + coefs[k][0];
      g = g * t + coefs[k][1];
      b = b * t + coefs[k][2];
    }
    out[i * 3 + 0] = Math.round(clamp01(r) * 255);
    out[i * 3 + 1] = Math.round(clamp01(g) * 255);
    out[i * 3 + 2] = Math.round(clamp01(b) * 255);
  }
  return out;
}

const VIRIDIS: readonly Vec3[] = [
  [0.2777273272234177, 0.005407344544966578, 0.3340998053353061],
  [0.1050930431085774, 1.404613529898575, 1.384590162594685],
  [-0.3308618287255563, 0.214847559468213, 0.09509516302823659],
  [-4.634230498983486, -5.799100973351585, -19.33244095627987],
  [6.228269936347081, 14.17993336680509, 56.69055260068105],
  [4.776384997670288, -13.74514537774601, -65.35303263337234],
  [-5.435455855934631, 4.645852612178535, 26.3124352495832],
];

const PLASMA: readonly Vec3[] = [
  [0.05873234392399702, 0.02333670892565664, 0.5433401826748754],
  [2.176514634195958, 0.2383834598145069, 0.7539604599784036],
  [-2.689460476458034, -7.455851135738909, 3.110799939717086],
  [6.130348345893603, 42.3461881477227, -28.51885465332158],
  [-11.10743619062271, -82.66631109428045, 60.13984767418263],
  [10.02306557647065, 71.41361770095349, -54.07218655560067],
  [-3.658713842777788, -22.93153465461149, 18.19190778539828],
];

const INFERNO: readonly Vec3[] = [
  [0.0002189403691192265, 0.001651004631001012, -0.01948089843709184],
  [0.1065134194856116, 0.5639564367884091, 3.932712388889277],
  [11.60249308247187, -3.972853965665698, -15.9423941062914],
  [-41.70399613139459, 17.43639888205313, 44.35414519872813],
  [77.162935699427, -33.40235894210092, -81.80730925738993],
  [-71.31942824499214, 32.62606426397723, 73.20951985803202],
  [25.13112622477341, -12.24266895238567, -23.07032500287172],
];

const MAGMA: readonly Vec3[] = [
  [-0.002136485053939582, -0.000749655052795221, -0.005386127855323933],
  [0.2516605407371642, 0.6775232436837668, 2.494026599312351],
  [8.353717279216625, -3.577719514958484, 0.3144679030132573],
  [-27.66873308576866, 14.26473078096533, -13.64921318813922],
  [52.17613981234068, -27.94360607168351, 12.94416944238394],
  [-50.76852536473588, 29.04658282127291, 4.23415299384855],
  [18.65570506591883, -11.48977351997711, -5.601961508734096],
];

/** Cividis isn't available in the shared polynomial set; use stop-based
 *  interpolation of its five published anchor colors. Perceptual
 *  characteristics aren't preserved exactly but the hue progression is. */
function buildCividis(): Uint8Array {
  const stops: readonly [number, Vec3][] = [
    [0.0, [0, 32, 77]],
    [0.25, [58, 66, 110]],
    [0.5, [125, 124, 120]],
    [0.75, [191, 169, 82]],
    [1.0, [255, 234, 70]],
  ];
  const out = new Uint8Array(256 * 3);
  for (let i = 0; i < 256; i++) {
    const t = i / 255;
    let lo = stops[0];
    let hi = stops[stops.length - 1];
    for (let s = 0; s < stops.length - 1; s++) {
      if (t >= stops[s][0] && t <= stops[s + 1][0]) {
        lo = stops[s];
        hi = stops[s + 1];
        break;
      }
    }
    const span = hi[0] - lo[0];
    const u = span > 0 ? (t - lo[0]) / span : 0;
    out[i * 3 + 0] = Math.round(lo[1][0] + (hi[1][0] - lo[1][0]) * u);
    out[i * 3 + 1] = Math.round(lo[1][1] + (hi[1][1] - lo[1][1]) * u);
    out[i * 3 + 2] = Math.round(lo[1][2] + (hi[1][2] - lo[1][2]) * u);
  }
  return out;
}

export type ColormapName = "viridis" | "plasma" | "inferno" | "magma" | "cividis";

export const COLORMAP_NAMES: ColormapName[] = [
  "inferno",
  "magma",
  "plasma",
  "viridis",
  "cividis",
];

export const DEFAULT_COLORMAP: ColormapName = "inferno";

const CACHE: Partial<Record<ColormapName, Uint8Array>> = {};

export function colormap(name: ColormapName): Uint8Array {
  if (CACHE[name]) return CACHE[name]!;
  let lut: Uint8Array;
  switch (name) {
    case "viridis": lut = polyLut(VIRIDIS); break;
    case "plasma":  lut = polyLut(PLASMA); break;
    case "inferno": lut = polyLut(INFERNO); break;
    case "magma":   lut = polyLut(MAGMA); break;
    case "cividis": lut = buildCividis(); break;
  }
  CACHE[name] = lut;
  return lut;
}
