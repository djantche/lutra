import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { onTrayBigPicture, setTrayLabels } from "../lib/backend";

export type UiMode = "classic" | "bigpicture";

const STORAGE_KEY = "lutra.mode";

function readStoredMode(): UiMode {
  try {
    return localStorage.getItem(STORAGE_KEY) === "bigpicture" ? "bigpicture" : "classic";
  } catch {
    return "classic";
  }
}

interface UiContextValue {
  mode: UiMode;
  setMode: (mode: UiMode) => void;
  toggleMode: () => void;
  /** The "add a game" dialog of the desktop mode. */
  addGameOpen: boolean;
  setAddGameOpen: (open: boolean) => void;
  /** Game whose settings window is open. */
  configureGameId: string | null;
  setConfigureGameId: (id: string | null) => void;
  /** Game waiting for the user to confirm its removal. */
  removeGameId: string | null;
  setRemoveGameId: (id: string | null) => void;
}

const UiContext = createContext<UiContextValue | null>(null);

export function UiProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<UiMode>(readStoredMode);
  const [addGameOpen, setAddGameOpen] = useState(false);
  const [configureGameId, setConfigureGameId] = useState<string | null>(null);
  const [removeGameId, setRemoveGameId] = useState<string | null>(null);

  const setMode = useCallback((next: UiMode) => {
    setModeState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Persistence is a convenience only.
    }
  }, []);

  const toggleMode = useCallback(() => setMode(mode === "classic" ? "bigpicture" : "classic"), [mode, setMode]);

  // The tray menu speaks the interface's language, and its "Big Picture" switches to it.
  const { t, i18n } = useTranslation();
  useEffect(() => {
    setTrayLabels({ open: t("tray.open"), bigPicture: t("nav.bigPicture"), quit: t("tray.quit") }).catch(() => {});
  }, [t, i18n.language]);
  useEffect(() => {
    const stop = onTrayBigPicture(() => setMode("bigpicture"));
    return () => void stop.then((unlisten) => unlisten());
  }, [setMode]);

  // `lutra --big-picture` forces console mode at startup.
  useEffect(() => {
    if (!isTauri()) return;
    invoke<UiMode>("launch_mode")
      .then((launchMode) => {
        if (launchMode === "bigpicture") setMode("bigpicture");
      })
      .catch(() => {});
  }, [setMode]);

  useEffect(() => {
    document.documentElement.dataset.mode = mode;
    if (!isTauri()) return;
    getCurrentWindow()
      .setFullscreen(mode === "bigpicture")
      .catch(() => {});
  }, [mode]);

  const value = useMemo(
    () => ({
      mode,
      setMode,
      toggleMode,
      addGameOpen,
      setAddGameOpen,
      configureGameId,
      setConfigureGameId,
      removeGameId,
      setRemoveGameId,
    }),
    [mode, setMode, toggleMode, addGameOpen, configureGameId, removeGameId],
  );
  return <UiContext.Provider value={value}>{children}</UiContext.Provider>;
}

export function useUi(): UiContextValue {
  const value = useContext(UiContext);
  if (!value) throw new Error("useUi must be used inside <UiProvider>");
  return value;
}
