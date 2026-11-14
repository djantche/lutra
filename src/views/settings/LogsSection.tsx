import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Dialog } from "../../components/Dialog/Dialog";
import { LogViewer } from "../../components/LogViewer/LogViewer";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { clearLogs, openLogsFolder } from "../../lib/backend";
import { reportError, useToastStore } from "../../state/toasts";
import styles from "./LogsSection.module.scss";

/** Access to Lutra's own log, the first place to look when something went wrong. */
export function LogsSection() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [busy, setBusy] = useState(false);

  const clear = () => {
    setBusy(true);
    clearLogs()
      .then(() => useToastStore.getState().notify("info", t("settings.logsCleared")))
      .catch((error) => reportError(t("errors.clearLog"), error))
      .finally(() => {
        setBusy(false);
        setClearing(false);
      });
  };

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
          <Button variant="ghost" icon="trash" onClick={() => setClearing(true)}>
            {t("settings.clearLogs")}
          </Button>
        </div>
      </SettingsRow>

      {clearing && (
        <Dialog
          title={t("settings.clearLogsTitle")}
          onClose={() => setClearing(false)}
          busy={busy}
          footer={
            <>
              <Button variant="ghost" onClick={() => setClearing(false)} disabled={busy}>
                {t("common.cancel")}
              </Button>
              <Button variant="primary" disabled={busy} onClick={clear}>
                {t("settings.clearLogs")}
              </Button>
            </>
          }
        >
          <p className={styles.text}>{t("settings.clearLogsHint")}</p>
        </Dialog>
      )}

      {open && (
        <Dialog size="xl" title={t("settings.appLog")} onClose={() => setOpen(false)}>
          <LogViewer target="app" live />
        </Dialog>
      )}
    </SettingsGroup>
  );
}
