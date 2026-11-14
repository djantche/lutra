import { hasBackend } from "./backend";
import { invoke, listen, type UnlistenFn } from "./host";

/** What `check_for_update` says about the latest GitHub release. */
export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  /** Release notes, in Markdown. */
  notes: string;
  page: string;
  published: string;
  /** Lutra can install it itself (an AppImage with a published checksum). */
  canInstall: boolean;
  size: number;
}

/** One `update-install` event. */
export interface UpdateInstallEvent {
  state: "download" | "verify" | "replace" | "done" | "failed" | "cancelled";
  received?: number;
  total?: number;
  /** The installed version when done, why it failed otherwise. */
  message?: string;
}

const MOCK: UpdateInfo = {
  current: "0.6.0",
  latest: "0.7.0",
  available: true,
  notes: "### Added\n\n- **Updates** from the About page.\n- A theme setting: light, dark or the system's.\n\n### Fixed\n\n- X and Y were swapped on some arcade sticks.",
  page: "https://github.com/djantche/lutra/releases/latest",
  published: "2026-11-20T10:00:00Z",
  canInstall: false,
  size: 132_042_656,
};

export const checkForUpdate = (): Promise<UpdateInfo> =>
  hasBackend() ? invoke<UpdateInfo>("check_for_update") : Promise.resolve(MOCK);

/** Starts the download; progress comes through `onUpdateInstall`. */
export const installUpdate = (): Promise<void> =>
  hasBackend() ? invoke<void>("install_update") : Promise.reject(new Error("Updating needs the Lutra window"));

export const cancelUpdate = (): Promise<void> => (hasBackend() ? invoke<void>("cancel_update") : Promise.resolve());

export function onUpdateInstall(callback: (event: UpdateInstallEvent) => void): Promise<UnlistenFn> {
  if (!hasBackend()) return Promise.resolve(() => {});
  return listen<UpdateInstallEvent>("update-install", (event) => callback(event.payload));
}

// ---- what the user chose, kept in this browser ----

const AUTO = "lutra.updates.auto";
const NOTIFY = "lutra.updates.notify";
const MAJOR_ONLY = "lutra.updates.majorOnly";
const SKIPPED = "lutra.updates.skipped";
const NOTIFIED = "lutra.updates.notified";
const CHECKED = "lutra.updates.checkedAt";

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

/** Look for a new version when Lutra starts, and once a day. On unless switched off. */
export const autoCheck = (): boolean => read(AUTO) !== "0";
export const setAutoCheck = (on: boolean) => write(AUTO, on ? "1" : "0");

/** Tell the user when one is found. On unless switched off. */
export const notifyUpdates = (): boolean => read(NOTIFY) !== "0";
export const setNotifyUpdates = (on: boolean) => write(NOTIFY, on ? "1" : "0");

/** Announce only major versions. Off unless switched on. */
export const majorOnly = (): boolean => read(MAJOR_ONLY) === "1";
export const setMajorOnly = (on: boolean) => write(MAJOR_ONLY, on ? "1" : "0");

/** The version the user asked not to be reminded about. */
export const skippedVersion = (): string | null => read(SKIPPED);
export const setSkippedVersion = (version: string | null) => write(SKIPPED, version);

/** The version already announced, so each one is announced once. */
export const notifiedVersion = (): string | null => read(NOTIFIED);
export const setNotifiedVersion = (version: string) => write(NOTIFIED, version);

export const lastChecked = (): number | null => {
  const value = Number(read(CHECKED));
  return Number.isFinite(value) && value > 0 ? value : null;
};
export const setLastChecked = (time: number) => write(CHECKED, String(time));

const numbers = (version: string): [number, number] | null => {
  const match = /^v?(\d+)\.(\d+)/.exec(version.trim());
  return match ? [Number(match[1]), Number(match[2])] : null;
};

/**
 * Whether going from `current` to `latest` is a major step: the first number goes up, or, while
 * it is still 0 (as semver has it for 0.y releases, where a new y may break things), the second.
 */
export function isMajorUpdate(current: string, latest: string): boolean {
  const from = numbers(current);
  const to = numbers(latest);
  if (!from || !to) return true;
  if (to[0] !== from[0]) return to[0] > from[0];
  return from[0] === 0 && to[1] > from[1];
}

/** Release notes as plain lines: Markdown's marks give way to what they meant. */
export function plainNotes(markdown: string): string {
  return markdown
    .replace(/\r/g, "")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/\*\*|__|`/g, "")
    .split("\n")
    .map((line) => line.replace(/^#{1,6}\s+/, "").replace(/^[-*]\s+/, "• "))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}
