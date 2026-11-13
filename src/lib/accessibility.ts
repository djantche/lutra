import { setZoom } from "./host";

// What the user chose for reading and moving, kept in this browser and applied as the app starts.

const SCALE = "lutra.scale";
const CONTRAST = "lutra.contrast";
const MOTION = "lutra.motion";

export const SCALE_MIN = 0.8;
export const SCALE_MAX = 1.5;

const read = (key: string): string | null => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

const write = (key: string, value: string | null) => {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // The choice only lasts for this session.
  }
};

/** The size of the interface: 1 is normal. */
export function uiScale(): number {
  const value = Number(read(SCALE));
  return Number.isFinite(value) && value >= SCALE_MIN && value <= SCALE_MAX ? value : 1;
}

export function highContrast(): boolean {
  return read(CONTRAST) === "1";
}

/** Whether motion is reduced: what the user chose, or else what the system asks for. */
export function reducedMotion(): boolean {
  const chosen = read(MOTION);
  if (chosen !== null) return chosen === "1";
  return matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Puts the saved choices to work. Called once as the interface starts. */
export function applyAccessibility(): void {
  setZoom(uiScale());
  document.documentElement.dataset.contrast = highContrast() ? "high" : "normal";
  document.documentElement.dataset.motion = reducedMotion() ? "reduced" : "full";
}

export function setUiScale(scale: number): void {
  write(SCALE, String(scale));
  applyAccessibility();
}

export function setHighContrast(on: boolean): void {
  write(CONTRAST, on ? "1" : "0");
  applyAccessibility();
}

export function setReducedMotion(on: boolean): void {
  write(MOTION, on ? "1" : "0");
  applyAccessibility();
}
