import { create } from "zustand";
import { checkForUpdate, installUpdate, setLastChecked, type UpdateInfo } from "../lib/updates";

export interface InstallProgress {
  state: "download" | "verify" | "replace";
  received: number;
  total: number;
}

interface UpdateStore {
  checking: boolean;
  info: UpdateInfo | null;
  error: string | null;
  /** The download in progress. */
  install: InstallProgress | null;
  /** Version in place on disk, waiting for a restart. */
  installed: string | null;
  failure: string | null;
  check: () => Promise<UpdateInfo | null>;
  startInstall: () => Promise<void>;
  setProgress: (progress: InstallProgress | null) => void;
  finish: (installed: string | null, failure: string | null) => void;
}

export const useUpdates = create<UpdateStore>((set, get) => ({
  checking: false,
  info: null,
  error: null,
  install: null,
  installed: null,
  failure: null,

  check: async () => {
    if (get().checking) return get().info;
    set({ checking: true, error: null });
    try {
      const info = await checkForUpdate();
      setLastChecked(Date.now());
      set({ info, checking: false });
      return info;
    } catch (error) {
      set({ checking: false, error: error instanceof Error ? error.message : String(error) });
      return null;
    }
  },

  startInstall: async () => {
    set({ failure: null, install: { state: "download", received: 0, total: get().info?.size ?? 0 } });
    try {
      await installUpdate();
    } catch (error) {
      set({ install: null, failure: error instanceof Error ? error.message : String(error) });
    }
  },

  setProgress: (install) => set({ install }),
  finish: (installed, failure) => set({ install: null, installed, failure }),
}));
