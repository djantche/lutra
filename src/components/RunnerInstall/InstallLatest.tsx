import { useState } from "react";
import { useTranslation } from "react-i18next";
import { availableRunners, installRunner } from "../../lib/runners";
import { useRunnerInstalls } from "../../state/runnerInstalls";
import { reportError } from "../../state/toasts";
import { Button } from "../Button/Button";
import { RunnerInstall } from "./RunnerInstall";
import styles from "./InstallLatest.module.scss";

/**
 * For a machine with no Wine or Proton: one button that downloads the latest GE-Proton, then the
 * progress of that install. The list of runners reloads itself when it finishes (`runners-changed`).
 */
export function InstallLatest() {
  const { t } = useTranslation();
  const jobs = useRunnerInstalls((state) => state.jobs);
  const [starting, setStarting] = useState(false);
  const job = Object.values(jobs).find((candidate) => candidate.name.startsWith("GE-Proton"));

  const start = () => {
    setStarting(true);
    availableRunners("proton-ge")
      .then((releases) => {
        const latest = releases.find((release) => !release.installed);
        if (!latest) throw new Error(t("runners.noRelease"));
        useRunnerInstalls.getState().dismiss(latest.name);
        return installRunner(latest.source, latest.tag);
      })
      .catch((error) => reportError(t("runners.startError"), error))
      .finally(() => setStarting(false));
  };

  if (job && !job.finished) return <RunnerInstall job={job} compact />;
  return (
    <div className={styles.offer}>
      <p>{t("runners.installLatestHint")}</p>
      <Button variant="primary" icon="download" disabled={starting} onClick={start}>
        {t("runners.installLatest")}
      </Button>
    </div>
  );
}
