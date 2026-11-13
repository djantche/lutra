import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  emulatorSettings,
  saveEmulatorSettings,
  DEFAULT_EMULATION_PROFILE,
  type CoreChoice,
  type EmulationProfile,
  type EmulatorConfig,
  type EmulatorSettings,
} from "../lib/backend";
import { useLibraryStore } from "../state/library";
import { reportError } from "../state/toasts";

/** The editable settings of an emulated game. Edits stay in memory until `save`. */
export function useEmulatorSettings(gameId: string, enabled: boolean, onLoadError: () => void) {
  const { t } = useTranslation();
  const upsertGame = useLibraryStore((state) => state.upsertGame);
  const [settings, setSettings] = useState<EmulatorSettings | null>(null);
  const [cores, setCores] = useState<CoreChoice[]>([]);
  const [consoleProfile, setConsoleProfile] = useState<EmulationProfile>(DEFAULT_EMULATION_PROFILE);
  const [saved, setSaved] = useState("");
  const [saving, setSaving] = useState(false);
  const failed = useRef(onLoadError);
  failed.current = onLoadError;

  useEffect(() => {
    if (!enabled) return;
    emulatorSettings(gameId)
      .then((view) => {
        setSettings(view.settings);
        setCores(view.cores);
        setConsoleProfile(view.consoleProfile);
        setSaved(JSON.stringify(view.settings));
      })
      .catch((error) => {
        reportError(t("errors.settings"), error);
        failed.current();
      });
  }, [gameId, enabled, t]);

  const dirty = settings !== null && JSON.stringify(settings) !== saved;

  const change = useCallback(
    (patch: Partial<EmulatorSettings>) => setSettings((current) => current && { ...current, ...patch }),
    [],
  );
  const changeConfig = useCallback(
    (patch: Partial<EmulatorConfig>) =>
      setSettings((current) => current && { ...current, config: { ...current.config, ...patch } }),
    [],
  );

  const save = useCallback(() => {
    if (!settings || saving) return;
    setSaving(true);
    saveEmulatorSettings(settings)
      .then((game) => {
        upsertGame(game);
        setSaved(JSON.stringify(settings));
      })
      .catch((error) => reportError(t("errors.save"), error))
      .finally(() => setSaving(false));
  }, [settings, saving, upsertGame, t]);

  return { settings, cores, consoleProfile, dirty, saving, change, changeConfig, save };
}
