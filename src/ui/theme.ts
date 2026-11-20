/**
 * UI theme: dark (the default vector-display look) or light. The choice persists in
 * localStorage, is mirrored onto <html data-theme="..."> for the CSS overrides, and is pushed
 * into the canvas palette via setPalette(). React components subscribe through the store
 * functions at the bottom (useSyncExternalStore-friendly).
 */
import { setPalette } from "./colors.ts";

export type Theme = "dark" | "light";

const KEY = "chimera-theme";
const listeners = new Set<() => void>();

function initial(): Theme {
  try {
    return localStorage.getItem(KEY) === "light" ? "light" : "dark";
  } catch {
    return "dark";
  }
}

let current: Theme = initial();

/** Apply the theme everywhere: canvas palette, <html> attribute, storage. */
export function setTheme(theme: Theme): void {
  if (theme === current) return;
  current = theme;
  setPalette(theme);
  document.documentElement.dataset.theme = theme;
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    /* private mode etc.: the in-memory choice still works */
  }
  listeners.forEach((cb) => cb());
}

export const toggleTheme = (): void => setTheme(current === "dark" ? "light" : "dark");
export const getTheme = (): Theme => current;
export const subscribeTheme = (cb: () => void): (() => void) => {
  listeners.add(cb);
  return () => listeners.delete(cb);
};

/**
 * Run once at startup before first paint. main.tsx imports this module for its side effect;
 * the attribute is also set here so a saved light choice does not flash a black screen.
 */
function boot(): void {
  setPalette(current);
  document.documentElement.dataset.theme = current;
}

boot();
