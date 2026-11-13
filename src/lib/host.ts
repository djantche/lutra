// The link with the shell around the interface (Electron): the backend's commands and events,
// the window, and the native dialogs. In a plain browser (`pnpm dev`) there is no shell and the
// app runs on mock data, so every caller checks `isHosted()` first.

interface PickOptions {
  directory?: boolean;
  filters?: { name: string; extensions: string[] }[];
}

interface LutraBridge {
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>;
  on(event: string, callback: (payload: unknown) => void): () => void;
  window(
    operation: "minimize" | "toggleMaximize" | "close" | "setFullscreen" | "toTray" | "quit",
    value?: boolean,
  ): Promise<void>;
  pick(options: PickOptions): Promise<string | null>;
  version(): Promise<string>;
  setZoom(factor: number): void;
}

declare global {
  interface Window {
    lutra?: LutraBridge;
  }
}

export type UnlistenFn = () => void;

/** True inside the Lutra window; false in a browser. */
export const isHosted = (): boolean => typeof window !== "undefined" && window.lutra !== undefined;

const bridge = (): LutraBridge => {
  if (!window.lutra) throw new Error("Lutra's shell is not available");
  return window.lutra;
};

/** Runs a backend command. Rejects with the backend's message. */
export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const result = await bridge().invoke(command, args);
  // Binary results (artwork) arrive as bytes.
  if (result instanceof Uint8Array) {
    return result.buffer.slice(result.byteOffset, result.byteOffset + result.byteLength) as T;
  }
  return result as T;
}

/** Listens to a backend event. The callback gets `{ payload }`. */
export function listen<T>(event: string, callback: (event: { payload: T }) => void): Promise<UnlistenFn> {
  return Promise.resolve(bridge().on(event, (payload) => callback({ payload: payload as T })));
}

/** The window the interface runs in. */
export const currentWindow = () => ({
  minimize: () => bridge().window("minimize"),
  toggleMaximize: () => bridge().window("toggleMaximize"),
  close: () => bridge().window("close"),
  setFullscreen: (on: boolean) => bridge().window("setFullscreen", on),
  /** Out of sight in the notification area (minimized when there is none). */
  hideToTray: () => bridge().window("toTray"),
  /** Quits Lutra. */
  quit: () => bridge().window("quit"),
});

/** Native file or folder picker. Resolves to the chosen path, or null when cancelled. */
export const pickPath = (options: PickOptions): Promise<string | null> => bridge().pick(options);

/** Zooms the whole interface; 1 is normal size. */
export function setZoom(factor: number): void {
  if (isHosted()) bridge().setZoom(factor);
  else document.documentElement.style.zoom = String(factor);
}

export const appVersion = (): Promise<string> => bridge().version();
