import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { currentWindow, invoke, isHosted } from "../lib/host";
import { useTranslation } from "react-i18next";
import { onTrayBigPicture, setTrayLabels } from "../lib/backend";

export type UiMode = "classic" | "bigpicture";

// The mode Lutra opens in, chosen in Settings. Switching modes while using the app (F11, the
// Big Picture button, the Guide menu) lasts for the session only. `lutra.mode` is what earlier
// versions saved at every switch: it still gives the default to whoever never chose one.
const DEFAULT_KEY = "lutra.defaultMode";
const LEGACY_KEY = "lutra.mode";

function readDefaultMode(): UiMode {
  try {
    const chosen = localStorage.getItem(DEFAULT_KEY) ?? localStorage.getItem(LEGACY_KEY);
    return chosen === "bigpicture" ? "bigpicture" : "classic";
  } catch {
    return "classic";
  }
}

interface UiContextValue {
  mode: UiMode;
  /** Switches now, for this session. */
  setMode: (mode: UiMode) => void;
  /** The mode Lutra opens in. */
  defaultMode: UiMode;
  setDefaultMode: (mode: UiMode) => void;
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
  /** The power menu (quit, minimize, switch the controller off...), opened by holding the Guide button. */
  powerMenuOpen: boolean;
  setPowerMenuOpen: (open: boolean) => void;
}

const UiContext = createContext<UiContextValue | null>(null);

export function UiProvider({ children }: { children: ReactNode }) {
  const [defaultMode, setDefaultModeState] = useState<UiMode>(readDefaultMode);
  const [mode, setMode] = useState<UiMode>(defaultMode);
  const [addGameOpen, setAddGameOpen] = useState(false);
  const [configureGameId, setConfigureGameId] = useState<string | null>(null);
  const [removeGameId, setRemoveGameId] = useState<string | null>(null);
  const [powerMenuOpen, setPowerMenuOpen] = useState(false);

  const setDefaultMode = useCallback((next: UiMode) => {
    setDefaultModeState(next);
    try {
      localStorage.setItem(DEFAULT_KEY, next);
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

  // `lutra --big-picture` opens in console mode this once, whatever the default is.
  useEffect(() => {
    if (!isHosted()) return;
    invoke<UiMode>("launch_mode")
      .then((launchMode) => {
        if (launchMode === "bigpicture") setMode("bigpicture");
      })
      .catch(() => {});
  }, [setMode]);

  useEffect(() => {
    document.documentElement.dataset.mode = mode;
    if (!isHosted()) return;
    currentWindow()
      .setFullscreen(mode === "bigpicture")
      .catch(() => {});
  }, [mode]);

  const value = useMemo(
    () => ({
      mode,
      setMode,
      defaultMode,
      setDefaultMode,
      toggleMode,
      addGameOpen,
      setAddGameOpen,
      configureGameId,
      setConfigureGameId,
      removeGameId,
      setRemoveGameId,
      powerMenuOpen,
      setPowerMenuOpen,
    }),
    [mode, defaultMode, setDefaultMode, toggleMode, addGameOpen, configureGameId, removeGameId, powerMenuOpen],
  );
  return <UiContext.Provider value={value}>{children}</UiContext.Provider>;
}

export function useUi(): UiContextValue {
  const value = useContext(UiContext);
  if (!value) throw new Error("useUi must be used inside <UiProvider>");
  return value;
}
