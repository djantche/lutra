import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  emulationStatus,
  emulationSystems,
  pickFolder,
  scanRoms,
  setEmulationDirs,
  type EmulationStatus,
  type SystemView,
} from "../lib/backend";
import { platformOf, type Platform } from "../lib/platforms";
import { useLibraryStore } from "../state/library";
import { reportError, useToastStore } from "../state/toasts";

/** A system as the emulation screens show it. */
export interface SystemEntry extends SystemView {
  platform: Platform;
  /** Games of this system in the library, counted live. */
  gameCount: number;
  state: "ready" | "bios" | "noCore";
}

function stateOf(system: SystemView): SystemEntry["state"] {
  if (system.cores.length === 0) return "noCore";
  const missing = system.firmware.some((file) => !file.optional && file.state === "missing");
  return missing ? "bios" : "ready";
}

/** The systems, Lutra's emulation setup, and the actions every emulation screen needs. */
export function useEmulation() {
  const { t } = useTranslation();
  const notify = useToastStore((state) => state.notify);
  const games = useLibraryStore((state) => state.games);
  const [systems, setSystems] = useState<SystemView[] | null>(null);
  const [status, setStatus] = useState<EmulationStatus | null>(null);
  const [scanning, setScanning] = useState(false);

  const reload = useCallback(() => {
    Promise.all([emulationSystems(), emulationStatus()])
      .then(([found, current]) => {
        setSystems(found);
        setStatus(current);
      })
      .catch((error) => reportError(t("errors.settings"), error));
  }, [t]);
  useEffect(reload, [reload]);

  const entries = useMemo<SystemEntry[] | null>(() => {
    if (!systems) return null;
    const counts = new Map<string, number>();
    for (const game of games) {
      if (game.system) counts.set(game.system, (counts.get(game.system) ?? 0) + 1);
    }
    return systems.map((system) => ({
      ...system,
      platform: platformOf(system.name),
      gameCount: counts.get(system.name) ?? 0,
      state: stateOf(system),
    }));
  }, [systems, games]);

  const scan = useCallback(() => {
    setScanning(true);
    return scanRoms()
      .then((summary) => {
        notify("info", t("emulation.scanned", { count: summary.total, added: summary.added, removed: summary.removed }));
        if (summary.unplaced > 0) notify("info", t("emulation.unplaced", { count: summary.unplaced }));
        if (summary.withoutCore > 0) notify("info", t("emulation.withoutCore", { count: summary.withoutCore }));
        reload();
      })
      .catch((error) => reportError(t("emulation.scanFailed"), error))
      .finally(() => setScanning(false));
  }, [notify, reload, t]);

  /** Asks for a folder of ROMs, keeps it, and looks through it. True once a folder was added. */
  const addRomFolder = useCallback(async (): Promise<boolean> => {
    if (!status) return false;
    try {
      const folder = await pickFolder();
      if (!folder || status.romDirs.includes(folder)) return false;
      const bios = status.biosDir === status.defaultBiosDir ? null : status.biosDir;
      await setEmulationDirs([...status.romDirs, folder], bios);
      await scan();
      return true;
    } catch (error) {
      reportError(t("errors.pick"), error);
      return false;
    }
  }, [scan, status, t]);

  return { systems: entries, status, reload, scan, scanning, addRomFolder };
}
