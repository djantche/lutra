import { hasBackend } from "./backend";
import { invoke, listen, type UnlistenFn } from "./host";

export type ComponentOperation = "install" | "enable" | "disable" | "remove";

/** While installing: fetching a file, or everything else (unpacking, registering...). */
export type ComponentPhase = "download" | "install";

/** A component offered for a game's prefix, and where it stands. */
export interface ComponentInfo {
  /** Also the key of its name and hint in the translations (`components.names.<verb>`). */
  verb: string;
  installed: boolean;
  /** Installed, but not the copy Lutra would install now: an update is offered. */
  outdated: boolean;
  /** Its DLLs are the real ones. Only meaningful once installed. */
  enabled: boolean;
  /** It has DLLs to switch (fonts have none). */
  canToggle: boolean;
  /** It can be taken out cleanly (the others ran an installer, which cannot be undone). */
  canRemove: boolean;
  /** What is being done to it right now. */
  operation: ComponentOperation | null;
  phase: ComponentPhase | null;
  /** How far the download is, 0 to 100, when it can be measured. */
  percent: number | null;
}

/** One `component-install` event: a job started, moved on, finished or failed. */
export interface ComponentEvent {
  id: string;
  verb: string;
  operation: ComponentOperation;
  state: "running" | "done" | "failed";
  message: string | null;
  phase: ComponentPhase;
  percent: number | null;
}

const MOCK_COMPONENTS: ComponentInfo[] = [
  ["d3dcompiler_47", true, true, true, true],
  ["vcrun2022", true, false, true, false],
  ["corefonts", false, false, false, false],
].map(([verb, installed, outdated, canToggle, canRemove]) => ({
  verb: verb as string,
  installed: installed as boolean,
  outdated: outdated as boolean,
  enabled: installed as boolean,
  canToggle: canToggle as boolean,
  canRemove: canRemove as boolean,
  operation: null,
  phase: null,
  percent: null,
}));

const needsLutra = () => Promise.reject(new Error("This needs the Lutra window"));

export const listComponents = (id: string): Promise<ComponentInfo[]> =>
  hasBackend() ? invoke<ComponentInfo[]>("list_components", { id }) : Promise.resolve(MOCK_COMPONENTS);

/** Installs or updates a component; how it goes comes through `onComponentInstall`. */
export const installComponent = (id: string, verb: string): Promise<void> =>
  hasBackend() ? invoke<void>("install_component", { id, verb }) : needsLutra();

/** Switches an installed component on or off. Nothing is deleted. */
export const setComponentEnabled = (id: string, verb: string, enabled: boolean): Promise<void> =>
  hasBackend() ? invoke<void>("set_component_enabled", { id, verb, enabled }) : needsLutra();

/** Takes a component out of the prefix, putting Wine's own DLLs back. */
export const removeComponent = (id: string, verb: string): Promise<void> =>
  hasBackend() ? invoke<void>("remove_component", { id, verb }) : needsLutra();

export function onComponentInstall(callback: (event: ComponentEvent) => void): Promise<UnlistenFn> {
  if (!hasBackend()) return Promise.resolve(() => {});
  return listen<ComponentEvent>("component-install", (event) => callback(event.payload));
}
