import { hasBackend } from "./backend";
import { invoke, listen, type UnlistenFn } from "./host";

/** Where a runner can be downloaded from. */
export type RunnerSource = "proton-ge" | "wine" | "wine-staging";

export const RUNNER_SOURCES: RunnerSource[] = ["proton-ge", "wine", "wine-staging"];

/** A release that can be installed. */
export interface AvailableRunner {
  source: RunnerSource;
  tag: string;
  /** The folder it installs to; also the key of its install. */
  name: string;
  size: number;
  published: string;
  installed: boolean;
}

/** A runner Lutra downloaded. */
export interface InstalledRunner {
  name: string;
  kind: "wine" | "proton";
  path: string;
  size: number;
  /** How many games are set to run with it. */
  games: number;
}

export type InstallStage = "prepare" | "download" | "verify" | "extract" | "done" | "failed" | "cancelled";

/** One `runner-install` event: a stage, how far it got, and a line for the log. */
export interface InstallEvent {
  name: string;
  stage: InstallStage;
  received: number;
  total: number;
  line: string | null;
}

const MOCK_AVAILABLE: AvailableRunner[] = [
  { source: "proton-ge", tag: "GE-Proton11-12", name: "GE-Proton11-12", size: 566_859_228, published: "2026-11-03T07:19:38Z", installed: false },
  { source: "proton-ge", tag: "GE-Proton11-11", name: "GE-Proton11-11", size: 564_789_552, published: "2026-10-19T18:20:53Z", installed: true },
  { source: "proton-ge", tag: "GE-Proton11-10", name: "GE-Proton11-10", size: 566_144_788, published: "2026-10-19T06:21:57Z", installed: false },
];
const MOCK_INSTALLED: InstalledRunner[] = [
  { name: "GE-Proton11-11", kind: "proton", path: "/mock/runners/GE-Proton11-11/proton", size: 1_810_000_000, games: 2 },
];

export const installedRunners = (): Promise<InstalledRunner[]> =>
  hasBackend() ? invoke<InstalledRunner[]>("installed_runners") : Promise.resolve(MOCK_INSTALLED);

export const availableRunners = (source: RunnerSource): Promise<AvailableRunner[]> =>
  hasBackend()
    ? invoke<AvailableRunner[]>("available_runners", { source })
    : Promise.resolve(MOCK_AVAILABLE.filter((runner) => runner.source === source));

/** Starts an install; progress comes through `onRunnerInstall`. */
export const installRunner = (source: RunnerSource, tag: string): Promise<void> =>
  hasBackend() ? invoke<void>("install_runner", { source, tag }) : Promise.reject(new Error("Installing needs the Lutra window"));

export const cancelRunnerInstall = (name: string): Promise<void> =>
  hasBackend() ? invoke<void>("cancel_runner_install", { name }) : Promise.resolve();

export const runningRunnerInstalls = (): Promise<string[]> =>
  hasBackend() ? invoke<string[]>("running_runner_installs") : Promise.resolve([]);

export const removeRunner = (name: string): Promise<void> =>
  hasBackend() ? invoke<void>("remove_runner", { name }) : Promise.resolve();

export function onRunnerInstall(callback: (event: InstallEvent) => void): Promise<UnlistenFn> {
  if (!hasBackend()) return Promise.resolve(() => {});
  return listen<InstallEvent>("runner-install", (event) => callback(event.payload));
}

/** The list of runners changed: one was installed or removed. */
export function onRunnersChanged(callback: () => void): Promise<UnlistenFn> {
  if (!hasBackend()) return Promise.resolve(() => {});
  return listen<null>("runners-changed", callback);
}
