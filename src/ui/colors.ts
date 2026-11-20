/**
 * The vector-display palette (sampled from the NORAD screens in WarGames; see docs/mockups).
 * Blue is the body and structure, amber is sound, and green/red/pink/white are the four control
 * slots: each screen maps its own four controls onto them and draws what they move in the same
 * colour. CSS mirrors these as custom properties in styles.css.
 *
 * The exported V / TEXT / C / SLOTS objects are mutable singletons: setPalette() swaps their
 * contents in place when the light theme is toggled, so every module that imported them picks
 * up the new colours on the next frame without re-importing.
 */
import type { Theme } from "./theme.ts";

export interface Palette {
  V: {
    blue: string;
    blueMid: string;
    blueDim: string;
    frame: string;
    green: string;
    red: string;
    pink: string;
    white: string;
    amber: string;
  };
  text: { primary: string; secondary: string };
  c: {
    black: string;
    console: string;
    inset: string;
    caliperDim: string;
    // MRI-like tissue intensities (T1-ish)
    fat: string;
    skin: string;
    muscle: string;
    tongue: string;
    brain: string;
    marrow: string;
    cortex: string;
    cartilage: string;
    mucosa: string;
    tooth: string;
  };
  /** sound level 0..1 stops for heat() */
  heat: [number, string][];
  /** fMRI-style acoustic energy stops for hot() */
  hot: [number, number, number, number][];
  /** colour the whitish/dark core of a hot beam mixes toward */
  coreTarget: string;
  /** canvas compositing used by beams (additive glow on black, plain overpaint on white) */
  blend: GlobalCompositeOperation;
}

const DARK: Palette = {
  V: {
    blue: "#4fb6f2",
    blueMid: "#3487c4",
    blueDim: "#1f5f8c",
    frame: "#133954",
    green: "#5be381",
    red: "#ff5d40",
    pink: "#ff73cf",
    white: "#eef6f8",
    amber: "#ffb340",
  },
  text: { primary: "#effbff", secondary: "#e2f5fc" },
  c: {
    black: "#000000",
    console: "#000000",
    inset: "#000000",
    caliperDim: "rgba(255, 179, 64, 0.45)",
    // MRI-like tissue intensities (T1-ish)
    fat: "#bdbdbd",
    skin: "#a8a8a8",
    muscle: "#5d5d5d",
    tongue: "#6b6b6b",
    brain: "#7c7c7c",
    marrow: "#b2b2b2",
    cortex: "#1c1c1c",
    cartilage: "#3e3e3e",
    mucosa: "#8f8f8f",
    tooth: "#101010",
  },
  heat: [[0, "#3a2408"], [0.35, "#c9822a"], [0.6, "#ffb340"], [0.82, "#ffe39a"], [1, "#fff8ec"]],
  hot: [
    [0.0, 0, 0, 0],
    [0.25, 138, 18, 0],
    [0.5, 240, 88, 28],
    [0.75, 255, 209, 92],
    [1.0, 255, 248, 230],
  ],
  coreTarget: "#ffffff",
  blend: "lighter",
};

const LIGHT: Palette = {
  V: {
    blue: "#0a6fc2",
    blueMid: "#1d5a92",
    blueDim: "#9cc7e6",
    frame: "#9fc3dd",
    green: "#1f9e4c",
    red: "#d83a1e",
    pink: "#d643a8",
    white: "#1a2a33",
    amber: "#c97a10",
  },
  text: { primary: "#0c2a3d", secondary: "#24475c" },
  c: {
    black: "#ffffff",
    console: "#ffffff",
    inset: "#eef1f3",
    caliperDim: "rgba(201, 122, 16, 0.4)",
    // tissue intensities inverted for a bright ground
    fat: "#5d5d5d",
    skin: "#6b6b6b",
    muscle: "#a8a8a8",
    tongue: "#9a9a9a",
    brain: "#8a8a8a",
    marrow: "#666666",
    cortex: "#e8e8e8",
    cartilage: "#c8c8c8",
    mucosa: "#7c7c7c",
    tooth: "#f2f2f2",
  },
  heat: [[0, "#f3e7d0"], [0.35, "#e0a357"], [0.6, "#c97a10"], [0.82, "#a35a00"], [1, "#6b3a00"]],
  hot: [
    [0.0, 255, 255, 255],
    [0.25, 255, 224, 205],
    [0.5, 240, 140, 90],
    [0.75, 200, 60, 30],
    [1.0, 140, 20, 0],
  ],
  coreTarget: "#000000",
  blend: "source-over",
};

export const PALETTES: Record<Theme, Palette> = { dark: DARK, light: LIGHT };

/** the vector line palette (mutated in place by setPalette) */
export const V = { ...DARK.V };
/** brighter canvas text, kept separate from the vector line palette */
export const TEXT = { ...DARK.text };
/** the four control slots, in order (refilled in place by setPalette) */
export const SLOTS: string[] = [V.green, V.red, V.pink, V.white];

/**
 * Older tokens, remapped onto the vector palette so screens not yet redrawn in the new style
 * sit on the console ground in blue and amber.
 */
export const C = {
  ...DARK.c,
  line: V.frame,
  text: V.blue,
  textDim: V.blueMid,
  caliper: V.amber,
};

/** current compositing mode for beams/dots/labels: additive in dark, plain in light */
export const blendMode = (): GlobalCompositeOperation => active.blend;

let active: Palette = DARK;
let heatStops: [number, string][] = DARK.heat;
let hotStops: [number, number, number, number][] = DARK.hot;
let coreTarget: string = DARK.coreTarget;

/** Swap every mutable palette object in place for the named theme. */
export function setPalette(theme: Theme): void {
  const p = PALETTES[theme];
  active = p;
  Object.assign(V, p.V);
  Object.assign(TEXT, p.text);
  SLOTS.splice(0, SLOTS.length, V.green, V.red, V.pink, V.white);
  Object.assign(C, p.c, { line: V.frame, text: V.blue, textDim: V.blueMid, caliper: V.amber });
  heatStops = p.heat;
  hotStops = p.hot;
  coreTarget = p.coreTarget;
}

/** mix two #rrggbb colours */
export function mix(hex: string, to: string, u: number): string {
  const a = parseInt(hex.slice(1), 16), b = parseInt(to.slice(1), 16);
  const ch = (s: number) => Math.round(((a >> s) & 255) * (1 - u) + ((b >> s) & 255) * u);
  return "#" + [16, 8, 0].map((s) => ch(s).toString(16).padStart(2, "0")).join("");
}
/** the bright (dark theme) or dark (light theme) core of a hot beam in this colour */
export const core = (hex: string): string => mix(hex, coreTarget, 0.55);

/** sound level 0..1 → ground amber → amber → pale yellow → white (dark), or the inverse on white */
export function heat(level: number): string {
  const v = Math.max(0, Math.min(1, level));
  for (let i = 1; i < heatStops.length; i++) {
    if (v <= heatStops[i][0]) {
      return mix(heatStops[i - 1][1], heatStops[i][1], (v - heatStops[i - 1][0]) / (heatStops[i][0] - heatStops[i - 1][0]));
    }
  }
  return heatStops[heatStops.length - 1][1];
}

export function hot(t: number): [number, number, number] {
  const ground = hotStops[0];
  if (!(t > 0)) return [ground[1], ground[2], ground[3]];
  const top = hotStops[hotStops.length - 1];
  if (t >= 1) return [top[1], top[2], top[3]];
  for (let i = 1; i < hotStops.length; i++) {
    if (t <= hotStops[i][0]) {
      const a = hotStops[i - 1], b = hotStops[i];
      const u = (t - a[0]) / (b[0] - a[0]);
      return [a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u, a[3] + (b[3] - a[3]) * u];
    }
  }
  return [top[1], top[2], top[3]];
}

export const hotCss = (t: number, alpha = 1): string => {
  const [r, g, b] = hot(t);
  return `rgba(${r | 0}, ${g | 0}, ${b | 0}, ${alpha})`;
};

/** Map acoustic pressure RMS (dyn/cm²) to 0..1 on a log scale. */
export const pressureLevel = (rms: number): number => {
  if (!(rms > 0)) return 0;
  const lo = Math.log10(40), hi = Math.log10(4000);
  return Math.max(0, Math.min(1, (Math.log10(rms) - lo) / (hi - lo)));
};
