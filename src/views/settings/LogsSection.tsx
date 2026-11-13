import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Dialog } from "../../components/Dialog/Dialog";
import { LogViewer } from "../../components/LogViewer/LogViewer";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { openLogsFolder } from "../../lib/backend";
import { reportError } from "../../state/toasts";
import styles from "./LogsSection.module.scss";

/** Access to Lutra's own log, the first place to look when something went wrong. */
export function LogsSection() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <SettingsGroup title={t("settings.groupLutraLog")}>
      <SettingsRow title={t("settings.appLog")} description={t("settings.appLogHint")}>
        <div className={styles.actions}>
          <Button variant="secondary" icon="terminal" onClick={() => setOpen(true)}>
            {t("settings.viewLog")}
          </Button>
          <Button
            variant="ghost"
            icon="folder"
            onClick={() => openLogsFolder().catch((error) => reportError(t("errors.folder"), error))}
          >
            {t("settings.openFolder")}
          </Button>
        </div>
      </SettingsRow>

      {open && (
        <Dialog size="xl" title={t("settings.appLog")} onClose={() => setOpen(false)}>
          <LogViewer target="app" live />
        </Dialog>
      )}
    </SettingsGroup>
  );
}
