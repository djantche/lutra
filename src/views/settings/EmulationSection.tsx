import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { FilePicker } from "../../components/FilePicker/FilePicker";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../components/Spinner/Spinner";
import {
  emulationStatus,
  openEmulationFolder,
  pickFolder,
  scanRoms,
  setEmulationDirs,
  type EmulationStatus,
} from "../../lib/backend";
import { useToastStore, reportError } from "../../state/toasts";
import styles from "./EmulationSection.module.scss";

/**
 * Where emulated games and BIOS files are. Lutra picks, installs and updates the emulator of
 * each console itself; consoles are on the Emulation page.
 */
export function EmulationSection() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<EmulationStatus | null>(null);
  const [scanning, setScanning] = useState(false);
  const notify = useToastStore((state) => state.notify);

  const load = useCallback(() => {
    emulationStatus()
      .then(setStatus)
      .catch((error) => reportError(t("errors.settings"), error));
  }, [t]);
  useEffect(load, [load]);

  const saveDirs = (romDirs: string[], biosDir: string | null) =>
    setEmulationDirs(romDirs, biosDir)
      .then(load)
      .catch((error) => reportError(t("errors.settings"), error));

  const scan = () => {
    setScanning(true);
    scanRoms()
      .then((summary) => {
        notify("info", t("emulation.scanned", { count: summary.total, added: summary.added, removed: summary.removed }));
        if (summary.unplaced > 0) notify("info", t("emulation.unplaced", { count: summary.unplaced }));
        if (summary.withoutCore > 0) notify("info", t("emulation.withoutCore", { count: summary.withoutCore }));
      })
      .catch((error) => reportError(t("emulation.scanFailed"), error))
      .finally(() => setScanning(false));
  };

  if (!status) {
    return (
      <SettingsGroup title={t("emulation.romFolders")}>
        <SettingsRow title={t("emulation.loading")}>
          <Spinner />
        </SettingsRow>
      </SettingsGroup>
    );
  }

  const addFolder = () =>
    pickFolder()
      .then((folder) => {
        if (folder && !status.romDirs.includes(folder)) return saveDirs([...status.romDirs, folder], customBios(status));
      })
      .catch((error) => reportError(t("errors.pick"), error));

  return (
    <>
      <SettingsGroup title={t("emulation.romFolders")}>
        <SettingsRow stacked title={t("emulation.romFoldersTitle")} description={t("emulation.romFoldersHint")}>
          {status.romDirs.length > 0 && (
            <ul className={styles.folders}>
              {status.romDirs.map((dir) => (
                <li key={dir}>
                  <span>{dir}</span>
                  <Button
                    variant="ghost"
                    onClick={() => saveDirs(status.romDirs.filter((other) => other !== dir), customBios(status))}
                  >
                    {t("common.remove")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <div className={styles.actions}>
            <Button variant="secondary" icon="plus" onClick={addFolder}>
              {t("emulation.addFolder")}
            </Button>
            <Button variant="primary" disabled={status.romDirs.length === 0 || scanning} onClick={scan}>
              {scanning && <Spinner />}
              {t("emulation.scan")}
            </Button>
          </div>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("emulation.bios")}>
        <SettingsRow stacked title={t("emulation.biosFolder")} description={t("emulation.biosFolderHint")}>
          <FilePicker
            value={status.biosDir}
            placeholder={t("config.prefixChoose")}
            onPick={() =>
              pickFolder()
                .then((folder) => {
                  if (folder) return saveDirs(status.romDirs, folder);
                })
                .catch((error) => reportError(t("errors.pick"), error))
            }
          />
          <div className={styles.actions}>
            {status.suggestedBiosDir && (
              <Button variant="secondary" onClick={() => saveDirs(status.romDirs, status.suggestedBiosDir)}>
                {t("emulation.useRetroarchBios", { path: status.suggestedBiosDir })}
              </Button>
            )}
            <Button
              variant="ghost"
              icon="folder"
              onClick={() => openEmulationFolder("bios").catch((error) => reportError(t("errors.folder"), error))}
            >
              {t("settings.openFolder")}
            </Button>
          </div>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("emulation.emulators")}>
        <SettingsRow title={t("emulation.emulatorsAuto")} description={t("emulation.emulatorsAutoHint")} />
        {!status.retroarch && (
          <SettingsRow title={t("emulation.notInstalled")} description={t("emulation.retroarchMissing")}>
            <span className={styles.warn}>RetroArch</span>
          </SettingsRow>
        )}
      </SettingsGroup>
    </>
  );
}

/** The BIOS folder to keep when only the ROM folders change: null when it is still the default. */
const customBios = (status: EmulationStatus): string | null =>
  status.biosDir === status.defaultBiosDir ? null : status.biosDir;
