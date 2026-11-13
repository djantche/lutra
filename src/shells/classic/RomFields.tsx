import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { FilePicker } from "../../components/FilePicker/FilePicker";
import { FormField } from "../../components/FormField/FormField";
import { PlatformPicker } from "../../components/PlatformPicker/PlatformPicker";
import { Spinner } from "../../components/Spinner/Spinner";
import type { SystemEntry } from "../../hooks/useEmulation";
import { installCore, pickRom } from "../../lib/backend";
import { reportError, useToastStore } from "../../state/toasts";
import styles from "./AddGameDialog.module.scss";

interface RomFieldsProps {
  systems: SystemEntry[] | null;
  system: string;
  onSystem: (system: string) => void;
  romPath: string;
  onRom: (path: string) => void;
  /** A core was installed: the systems must be read again. */
  onInstalled: () => void;
  /** Adds a whole folder of ROMs instead of one game. */
  onAddFolder: () => void;
}

/** Console and file of a ROM added by hand. The emulator is Lutra's choice for the console. */
export function RomFields({ systems, system, onSystem, romPath, onRom, onInstalled, onAddFolder }: RomFieldsProps) {
  const { t } = useTranslation();
  const notify = useToastStore((state) => state.notify);
  const [installing, setInstalling] = useState(false);
  const chosen = systems?.find((entry) => entry.name === system);

  if (!systems) {
    return (
      <p className={styles.note}>
        <Spinner /> {t("emulation.loading")}
      </p>
    );
  }

  const browse = () =>
    pickRom(chosen?.extensions ?? [])
      .then((picked) => picked && onRom(picked))
      .catch((error) => reportError(t("errors.pick"), error));

  const install = (id: string) => {
    setInstalling(true);
    installCore(id)
      .then((probe) => {
        notify("info", t("emulation.installed", { name: probe.name, version: probe.version }));
        onInstalled();
      })
      .catch((error) => reportError(t("emulation.installFailed", { name: id }), error))
      .finally(() => setInstalling(false));
  };

  const missingBios = chosen?.firmware.filter((file) => !file.optional && file.state === "missing") ?? [];

  return (
    <>
      <FormField label={t("addGame.console")}>
        <PlatformPicker systems={systems} value={system} onChange={onSystem} />
      </FormField>

      {chosen && (
        <>
          <FormField
            label={t("addGame.romFile")}
            hint={
              chosen.extensions.length > 0
                ? t("addGame.romFormats", { formats: chosen.extensions.map((ext) => `.${ext}`).join(" ") })
                : undefined
            }
          >
            <FilePicker value={romPath} placeholder={t("addGame.browse")} onPick={browse} />
          </FormField>

          {chosen.cores.length === 0 && (
            <div className={styles.warning}>
              <span>{chosen.recommendedCore ? t("emulation.noCoreAuto") : t("emulation.noCoreYet")}</span>
              {chosen.recommendedCore && (
                <Button
                  variant="secondary"
                  disabled={installing}
                  onClick={() => chosen.recommendedCore && install(chosen.recommendedCore)}
                >
                  {installing && <Spinner />}
                  {t("emulation.installNow")}
                </Button>
              )}
            </div>
          )}

          {missingBios.length > 0 && (
            <p className={styles.warning}>
              {t("addGame.biosMissing", { files: missingBios.map((file) => file.path).join(", ") })}
            </p>
          )}
        </>
      )}

      {romPath === "" && (
        <p className={styles.note}>
          {t("addGame.folderInstead")}{" "}
          <Button variant="ghost" icon="folder" onClick={onAddFolder}>
            {t("emulation.addFolder")}
          </Button>
        </p>
      )}
    </>
  );
}
