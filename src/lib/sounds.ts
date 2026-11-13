// Interface sounds, a few short ones like a console's menus. See `assets/sounds/CREDITS.md`.
import back from "../assets/sounds/back.wav?url";
import close from "../assets/sounds/close.wav?url";
import confirm from "../assets/sounds/confirm.wav?url";
import error from "../assets/sounds/error.wav?url";
import move from "../assets/sounds/move.wav?url";
import off from "../assets/sounds/off.wav?url";
import on from "../assets/sounds/on.wav?url";
import open from "../assets/sounds/open.wav?url";

export type SoundName = "move" | "confirm" | "back" | "on" | "off" | "open" | "close" | "error";

const FILES: Record<SoundName, string> = { move, confirm, back, on, off, open, close, error };
/** The cursor moves often and must stay in the background. */
const GAIN: Record<SoundName, number> = { move: 0.35, confirm: 0.6, back: 0.5, on: 0.5, off: 0.5, open: 0.4, close: 0.4, error: 0.5 };
/** The same sound is not played again within this time: holding a direction is not a machine gun. */
const MIN_GAP_MS = 45;

const STORAGE_KEY = "lutra.sounds";

let enabled = (() => {
  try {
    return localStorage.getItem(STORAGE_KEY) !== "0";
  } catch {
    return true;
  }
})();

let context: AudioContext | null = null;
const buffers = new Map<SoundName, AudioBuffer>();
const lastPlayed = new Map<SoundName, number>();

export const soundsEnabled = (): boolean => enabled;

// Loaded at start, so the first sound is not lost to the loading.
if (enabled && typeof window !== "undefined") void load();

export function setSoundsEnabled(on: boolean): void {
  enabled = on;
  try {
    localStorage.setItem(STORAGE_KEY, on ? "1" : "0");
  } catch {
    // The choice only lasts for this session.
  }
  if (on) void load();
}

async function load(): Promise<void> {
  if (typeof AudioContext === "undefined") return;
  context ??= new AudioContext();
  await Promise.all(
    (Object.keys(FILES) as SoundName[]).map(async (name) => {
      if (buffers.has(name)) return;
      try {
        const data = await (await fetch(FILES[name])).arrayBuffer();
        buffers.set(name, await context!.decodeAudioData(data));
      } catch {
        // A sound that cannot be loaded is simply not played.
      }
    }),
  );
}

export function playSound(name: SoundName): void {
  if (!enabled) return;
  if (!context) {
    void load();
    return;
  }
  const buffer = buffers.get(name);
  const now = performance.now();
  if (!buffer || now - (lastPlayed.get(name) ?? 0) < MIN_GAP_MS) return;
  lastPlayed.set(name, now);
  if (context.state === "suspended") void context.resume();
  const source = context.createBufferSource();
  const gain = context.createGain();
  gain.gain.value = GAIN[name];
  source.buffer = buffer;
  source.connect(gain).connect(context.destination);
  source.start();
}
