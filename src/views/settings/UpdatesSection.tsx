import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { ProgressBar } from "../../components/ProgressBar/ProgressBar";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../components/Spinner/Spinner";
import { Switch } from "../../components/Switch/Switch";
import { openExternal } from "../../lib/backend";
import { formatBytes } from "../../lib/format";
import { relaunchApp } from "../../lib/host";
import {
  autoCheck,
  cancelUpdate,
  lastChecked,
  majorOnly,
  notifyUpdates,
  plainNotes,
  setAutoCheck,
  setMajorOnly,
  setNotifyUpdates,
  setSkippedVersion,
  skippedVersion,
} from "../../lib/updates";
import { useUpdates } from "../../state/updates";
import { reportError } from "../../state/toasts";
import styles from "./UpdatesSection.module.scss";

/** Looking for a new Lutra release on GitHub, installing it, and how to be told about one. */
export function UpdatesSection({ version }: { version: string | null }) {
  const { t, i18n } = useTranslation();
  const { checking, info, error, install, installed, failure, check, startInstall } = useUpdates();
  const [auto, setAuto] = useState(autoCheck);
  const [notify, setNotify] = useState(notifyUpdates);
  const [major, setMajor] = useState(majorOnly);
  const [skipped, setSkipped] = useState(skippedVersion);

  const checkedAt = lastChecked();
  const size = (bytes: number) => formatBytes(bytes, i18n.language);

  const status = checking
    ? t("updates.checking")
    : error
      ? t("updates.checkFailed", { reason: error })
      : info?.available
        ? t("updates.available", { version: info.latest })
        : info
          ? t("updates.upToDate")
          : t("updates.idle");

  const skip = (on: boolean) => {
    const next = on && info ? info.latest : null;
    setSkippedVersion(next);
    setSkipped(next);
  };

  const progress = install && install.total > 0 && install.state === "download" ? install.received / install.total : null;

  return (
    <>
      <SettingsGroup title={t("updates.title")}>
        <SettingsRow
          title={version ? t("updates.running", { version }) : t("updates.title")}
          description={
            checkedAt && !checking
              ? `${status} · ${t("updates.checkedAt", { time: new Date(checkedAt).toLocaleString(i18n.language, { dateStyle: "medium", timeStyle: "short" }) })}`
              : status
          }
        >
          <Button variant="secondary" icon="refresh" disabled={checking || install !== null} onClick={() => void check()}>
            {checking && <Spinner />}
            {t("updates.check")}
          </Button>
        </SettingsRow>

        {info?.available && (
          <SettingsRow
            stacked
            title={t("updates.version", { version: info.latest })}
            description={new Date(info.published).toLocaleDateString(i18n.language, { dateStyle: "long" })}
          >
            <div className={styles.release}>
              {info.notes.trim() !== "" && (
                <pre className={styles.notes} tabIndex={0}>
                  {plainNotes(info.notes)}
                </pre>
              )}

              {installed ? (
                <div className={styles.actions}>
                  <span className={styles.done}>{t("updates.installed", { version: installed })}</span>
                  <Button variant="primary" icon="refresh" onClick={() => relaunchApp().catch((e) => reportError(t("updates.restartError"), e))}>
                    {t("updates.restart")}
                  </Button>
                </div>
              ) : install ? (
                <div className={styles.progress}>
                  <ProgressBar value={progress} ariaLabel={t("updates.downloading")} />
                  <div className={styles.actions}>
                    <span className={styles.status}>
                      {install.state === "download"
                        ? `${t("updates.downloading")} · ${size(install.received)} / ${size(install.total)}`
                        : t(`updates.${install.state}`)}
                    </span>
                    {install.state === "download" && (
                      <Button variant="ghost" icon="close" onClick={() => cancelUpdate().catch((e) => reportError(t("updates.cancelError"), e))}>
                        {t("updates.cancel")}
                      </Button>
                    )}
                  </div>
                </div>
              ) : (
                <div className={styles.actions}>
                  {info.canInstall ? (
                    <Button variant="primary" icon="download" onClick={() => void startInstall()}>
                      {t("updates.install", { size: size(info.size) })}
                    </Button>
                  ) : (
                    <Button variant="primary" icon="external" onClick={() => void openExternal(info.page)}>
                      {t("updates.openPage")}
                    </Button>
                  )}
                  {info.canInstall && (
                    <Button variant="ghost" icon="external" onClick={() => void openExternal(info.page)}>
                      {t("updates.openPage")}
                    </Button>
                  )}
                  <Button variant="ghost" onClick={() => skip(skipped !== info.latest)}>
                    {skipped === info.latest ? t("updates.unskip") : t("updates.skip")}
                  </Button>
                </div>
              )}

              {!info.canInstall && !install && !installed && <p className={styles.hint}>{t("updates.manualHint")}</p>}
              {failure !== null && <p className={styles.failure}>{t("updates.failed", { reason: failure })}</p>}
            </div>
          </SettingsRow>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("updates.options")}>
        <SettingsRow title={t("updates.auto")} description={t("updates.autoHint")}>
          <Switch
            ariaLabel={t("updates.auto")}
            checked={auto}
            onChange={(on) => {
              setAuto(on);
              setAutoCheck(on);
            }}
          />
        </SettingsRow>
        <SettingsRow title={t("updates.notify")} description={t("updates.notifyHint")}>
          <Switch
            ariaLabel={t("updates.notify")}
            checked={auto && notify}
            disabled={!auto}
            onChange={(on) => {
              setNotify(on);
              setNotifyUpdates(on);
            }}
          />
        </SettingsRow>
        <SettingsRow title={t("updates.major")} description={t("updates.majorHint")}>
          <Switch
            ariaLabel={t("updates.major")}
            checked={auto && notify && major}
            disabled={!auto || !notify}
            onChange={(on) => {
              setMajor(on);
              setMajorOnly(on);
            }}
          />
        </SettingsRow>
      </SettingsGroup>
    </>
  );
}
