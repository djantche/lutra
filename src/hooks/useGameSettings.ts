import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  detailsFromGame,
  isValidDate,
  metaFromDetails,
  type DetailsForm,
} from "../components/GameDetailsFields/details";
import {
  listRunners,
  saveGameMeta,
  saveWineSettings,
  wineSettings,
  wineTools,
  type Runner,
  type Tools,
  type WineConfig,
  type WineSettings,
} from "../lib/backend";
import { useLibraryStore } from "../state/library";
import { reportError } from "../state/toasts";

/**
 * The editable settings of a game added by hand (Windows or Linux): what the launch uses plus the
 * details shown in the library. Edits stay in memory until `save`, which writes both.
 */
export function useGameSettings(gameId: string, enabled: boolean, onLoadError: () => void) {
  const { t } = useTranslation();
  const upsertGame = useLibraryStore((state) => state.upsertGame);
  const [settings, setSettings] = useState<WineSettings | null>(null);
  const [details, setDetails] = useState<DetailsForm | null>(null);
  // What was loaded or last saved, to tell whether anything changed.
  const [saved, setSaved] = useState("");
  const [runners, setRunners] = useState<Runner[]>([]);
  const [tools, setTools] = useState<Tools | null>(null);
  const [saving, setSaving] = useState(false);
  const failed = useRef(onLoadError);
  failed.current = onLoadError;

  useEffect(() => {
    if (!enabled) return;
    const game = useLibraryStore.getState().games.find((candidate) => candidate.id === gameId);
    wineSettings(gameId)
      .then((loaded) => {
        const loadedDetails = game ? detailsFromGame(game) : null;
        setSettings(loaded);
        setDetails(loadedDetails);
        setSaved(JSON.stringify([loaded, loadedDetails]));
      })
      .catch((error) => {
        reportError(t("errors.settings"), error);
        failed.current();
      });
    listRunners()
      .then(setRunners)
      .catch(() => setRunners([]));
    wineTools()
      .then(setTools)
      .catch(() => setTools(null));
  }, [gameId, enabled, t]);

  const dirty = settings !== null && JSON.stringify([settings, details]) !== saved;
  const valid = details === null || isValidDate(details.releaseDate);

  const change = useCallback(
    (patch: Partial<WineSettings>) => setSettings((current) => current && { ...current, ...patch }),
    [],
  );
  const changeConfig = useCallback(
    (patch: Partial<WineConfig>) =>
      setSettings((current) => current && { ...current, config: { ...current.config, ...patch } }),
    [],
  );

  const save = useCallback(() => {
    if (!settings || !valid || saving) return;
    setSaving(true);
    saveWineSettings(settings)
      .then((result) => (details ? saveGameMeta(gameId, metaFromDetails(details)) : result))
      .then((result) => {
        upsertGame(result);
        setSaved(JSON.stringify([settings, details]));
      })
      .catch((error) => reportError(t("errors.save"), error))
      .finally(() => setSaving(false));
  }, [settings, details, valid, saving, gameId, upsertGame, t]);

  return { settings, details, setDetails, runners, tools, dirty, valid, saving, change, changeConfig, save };
}
