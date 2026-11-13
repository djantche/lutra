import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Dialog } from "../../components/Dialog/Dialog";
import { Icon } from "../../components/Icon/Icon";
import { RunnerInstall } from "../../components/RunnerInstall/RunnerInstall";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../components/Spinner/Spinner";
import { formatBytes } from "../../lib/format";
import {
  availableRunners,
  installedRunners,
  installRunner,
  onRunnersChanged,
  removeRunner,
  RUNNER_SOURCES,
  type AvailableRunner,
  type InstalledRunner,
  type RunnerSource,
} from "../../lib/runners";
import { useRunnerInstalls } from "../../state/runnerInstalls";
import { reportError } from "../../state/toasts";
import styles from "./RunnersSection.module.scss";

/** How many releases of a source are listed. */
const SHOWN = 10;

/**
 * Wine and Proton builds Lutra downloads and keeps itself: what is installed, what can be
 * installed (GE-Proton, Wine, Wine Staging), and the installs in progress with their real progress.
 */
export function RunnersSection() {
  const { t, i18n } = useTranslation();
  const jobs = useRunnerInstalls((state) => state.jobs);
  const [installed, setInstalled] = useState<InstalledRunner[] | null>(null);
  const [source, setSource] = useState<RunnerSource>("proton-ge");
  const [available, setAvailable] = useState<AvailableRunner[] | null>(null);
  const [offline, setOffline] = useState<string | null>(null);
  const [removing, setRemoving] = useState<InstalledRunner | null>(null);

  const loadInstalled = useCallback(() => {
    installedRunners()
      .then(setInstalled)
      .catch((error) => reportError(t("runners.listError"), error));
  }, [t]);

  const loadAvailable = useCallback(() => {
    setAvailable(null);
    setOffline(null);
    availableRunners(source)
      .then(setAvailable)
      .catch((error) => {
        setAvailable([]);
        setOffline(String(error instanceof Error ? error.message : error));
      });
  }, [source]);

  useEffect(loadInstalled, [loadInstalled]);
  useEffect(loadAvailable, [loadAvailable]);
  // Installing or removing changes both lists.
  useEffect(() => {
    const stop = onRunnersChanged(() => {
      loadInstalled();
      loadAvailable();
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [loadInstalled, loadAvailable]);

  const start = (runner: AvailableRunner) => {
    // The previous run of the same install, if it ended, is replaced by this one.
    useRunnerInstalls.getState().dismiss(runner.name);
    installRunner(runner.source, runner.tag).catch((error) => reportError(t("runners.startError"), error));
  };

  const confirmRemove = () => {
    const target = removing;
    setRemoving(null);
    if (!target) return;
    removeRunner(target.name).catch((error) => reportError(t("runners.removeError"), error));
  };

  const size = (bytes: number) => formatBytes(bytes, i18n.language);
  const date = (iso: string) => new Date(iso).toLocaleDateString(i18n.language, { dateStyle: "medium" });
  const jobList = Object.values(jobs);

  return (
    <>
      {jobList.length > 0 && (
        <SettingsGroup title={t("runners.progress")}>
          {jobList.map((job) => (
            <RunnerInstall key={job.name} job={job} />
          ))}
        </SettingsGroup>
      )}

      <SettingsGroup title={t("runners.installedTitle")}>
        {installed === null && (
          <SettingsRow title={t("runners.loading")}>
            <Spinner />
          </SettingsRow>
        )}
        {installed?.length === 0 && (
          <SettingsRow title={t("runners.noneTitle")} description={t("runners.noneHint")} />
        )}
        {installed?.map((runner) => (
          <SettingsRow
            key={runner.name}
            title={runner.name}
            description={[
              t(`runners.kind.${runner.kind}`),
              size(runner.size),
              runner.games > 0 ? t("runners.usedBy", { count: runner.games }) : t("runners.unused"),
            ].join(" · ")}
          >
            <Button variant="ghost" icon="trash" onClick={() => setRemoving(runner)}>
              {t("runners.remove")}
            </Button>
          </SettingsRow>
        ))}
      </SettingsGroup>

      <SettingsGroup title={t("runners.installTitle")}>
        <SettingsRow title={t("runners.source")} description={t(`runners.sourceHint.${source}`)}>
          <div className={styles.source}>
            <SegmentedControl
              ariaLabel={t("runners.source")}
              value={source}
              onChange={setSource}
              segments={RUNNER_SOURCES.map((value) => ({ value, label: t(`runners.sources.${value}`) }))}
            />
            <Button variant="ghost" icon="refresh" aria-label={t("runners.refresh")} onClick={loadAvailable} />
          </div>
        </SettingsRow>

        {available === null && (
          <SettingsRow title={t("runners.fetching")}>
            <Spinner />
          </SettingsRow>
        )}
        {offline && (
          <SettingsRow title={t("runners.offlineTitle")} description={`${t("runners.offlineHint")} ${offline}`}>
            <Button variant="secondary" icon="refresh" onClick={loadAvailable}>
              {t("runners.retry")}
            </Button>
          </SettingsRow>
        )}
        {available?.slice(0, SHOWN).map((runner, index) => {
          const job = jobs[runner.name];
          const running = job !== undefined && !job.finished;
          return (
            <SettingsRow
              key={runner.name}
              title={runner.tag}
              description={[date(runner.published), size(runner.size)].join(" · ")}
            >
              <div className={styles.action}>
                {index === 0 && <span className={styles.latest}>{t("runners.latest")}</span>}
                {runner.installed ? (
                  <span className={styles.done}>
                    <Icon name="check" size={16} />
                    {t("runners.alreadyInstalled")}
                  </span>
                ) : running ? (
                  <span className={styles.done}>
                    <Spinner />
                    {t("runners.installing")}
                  </span>
                ) : (
                  <Button variant={index === 0 ? "primary" : "secondary"} icon="download" onClick={() => start(runner)}>
                    {t("runners.install")}
                  </Button>
                )}
              </div>
            </SettingsRow>
          );
        })}
      </SettingsGroup>

      {removing && (
        <Dialog
          title={t("runners.removeTitle", { name: removing.name })}
          description={removing.games > 0 ? t("runners.removeUsed", { count: removing.games }) : t("runners.removeHint", { size: size(removing.size) })}
          onClose={() => setRemoving(null)}
          footer={
            <>
              <Button variant="ghost" onClick={() => setRemoving(null)}>
                {t("common.cancel")}
              </Button>
              <Button variant="primary" icon="trash" onClick={confirmRemove}>
                {t("runners.remove")}
              </Button>
            </>
          }
        >
          {null}
        </Dialog>
      )}
    </>
  );
}
