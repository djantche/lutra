import { hasBackend } from "./backend";
import { invoke, listen, type UnlistenFn } from "./host";

export type ToolId = "wine" | "umu" | "winetricks" | "gamemode" | "mangohud";

/** How a helper program can be installed on this system. */
export interface ToolPlan {
  tool: ToolId;
  /** What a person would type as administrator; null when the system has no package for it. */
  command: string | null;
  /** Lutra can run it itself (the system asks for the password in its own dialog). */
  canRun: boolean;
}

/** One `tool-install` event: a line of the package manager's output, or the end of the install. */
export interface ToolInstallEvent {
  tool: ToolId;
  state: "running" | "done" | "failed" | "cancelled";
  line?: string;
  /** Why it failed: `no_authentication`, or the manager's exit code. */
  message?: string;
}

const MOCK_PLANS: ToolPlan[] = [
  { tool: "wine", command: "sudo pacman -S --needed --noconfirm wine", canRun: true },
  { tool: "umu", command: "sudo pacman -S --needed --noconfirm umu-launcher", canRun: true },
  { tool: "winetricks", command: "sudo pacman -S --needed --noconfirm winetricks", canRun: true },
  { tool: "gamemode", command: "sudo pacman -S --needed --noconfirm gamemode", canRun: true },
  { tool: "mangohud", command: "sudo pacman -S --needed --noconfirm mangohud", canRun: false },
];

export const toolInstallPlans = (): Promise<ToolPlan[]> =>
  hasBackend() ? invoke<ToolPlan[]>("tool_install_plans") : Promise.resolve(MOCK_PLANS);

/** Starts an install; progress comes through `onToolInstall`. */
export const installTool = (tool: ToolId): Promise<void> =>
  hasBackend() ? invoke<void>("install_tool", { tool }) : Promise.reject(new Error("Installing needs the Lutra window"));

/** The tool being installed right now (the page may open in the middle of one). */
export const installingTool = (): Promise<ToolId | null> =>
  hasBackend() ? invoke<ToolId | null>("installing_tool") : Promise.resolve(null);

export function onToolInstall(callback: (event: ToolInstallEvent) => void): Promise<UnlistenFn> {
  if (!hasBackend()) return Promise.resolve(() => {});
  return listen<ToolInstallEvent>("tool-install", (event) => callback(event.payload));
}

/** Where to get a tool that no package manager here offers. */
export const TOOL_HOMEPAGES: Partial<Record<ToolId, string>> = {
  umu: "https://github.com/Open-Wine-Components/umu-launcher",
};
