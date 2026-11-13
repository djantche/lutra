import { useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cx } from "../../lib/cx";
import { formatBytes } from "../../lib/format";
import { cancelRunnerInstall } from "../../lib/runners";
import { jobProgress, useRunnerInstalls, type InstallJob } from "../../state/runnerInstalls";
import { reportError } from "../../state/toasts";
import { Button } from "../Button/Button";
import { ProgressBar } from "../ProgressBar/ProgressBar";
import styles from "./RunnerInstall.module.scss";

/** "2 min 05 s", "48 s". */
function duration(seconds: number): string {
  const whole = Math.max(1, Math.round(seconds));
  if (whole < 60) return `${whole} s`;
  return `${Math.floor(whole / 60)} min ${String(whole % 60).padStart(2, "0")} s`;
}

/** What the install is doing right now, in words, with the numbers that are really known. */
function useStatus(job: InstallJob): string {
  const { t, i18n } = useTranslation();
  const size = (bytes: number) => formatBytes(bytes, i18n.language);
  switch (job.stage) {
    case "download": {
      const parts = [t("runners.stage.download"), `${size(job.received)} / ${size(job.total)}`];
      if (job.speed > 0) {
        parts.push(`${size(job.speed)}/s`);
        parts.push(t("runners.eta", { time: duration((job.total - job.received) / job.speed) }));
      }
      return parts.join(" · ");
    }
    case "extract":
      return job.total > 0
        ? `${t("runners.stage.extract")} · ${t("runners.files", { done: job.received, total: job.total })}`
        : t("runners.stage.extract");
    case "failed":
      return `${t("runners.stage.failed")} · ${job.log[job.log.length - 1] ?? ""}`;
    default:
      return t(`runners.stage.${job.stage}`);
  }
}

/** One install: its stage, a bar that follows the real work, a way to stop it, and its log. */
export function RunnerInstall({ job, compact = false }: { job: InstallJob; compact?: boolean }) {
  const { t } = useTranslation();
  const status = useStatus(job);
  const progress = jobProgress(job);
  const [showLog, setShowLog] = useState(!compact);
  const scroller = useRef<HTMLPreElement>(null);

  // Stays on the newest line.
  useLayoutEffect(() => {
    const element = scroller.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [job.log.length, showLog]);

  const stop = () => cancelRunnerInstall(job.name).catch((error) => reportError(t("runners.cancelError"), error));

  return (
    <div className={cx(styles.card, job.stage === "failed" && styles.failed)}>
      <div className={styles.head}>
        <strong className={styles.name}>{job.name}</strong>
        <span className={styles.status}>{status}</span>
      </div>
      <div className={styles.bar}>
        <ProgressBar value={job.finished && job.stage !== "done" ? (progress ?? 0) : progress} ariaLabel={job.name} failed={job.stage === "failed"} />
        {progress !== null && <span className={styles.percent}>{Math.round(progress * 100)} %</span>}
        <Button variant="ghost" icon="terminal" aria-pressed={showLog} onClick={() => setShowLog((on) => !on)}>
          {t("runners.log")}
        </Button>
        {job.finished ? (
          <Button variant="secondary" onClick={() => useRunnerInstalls.getState().dismiss(job.name)}>
            {t("runners.dismiss")}
          </Button>
        ) : (
          <Button variant="secondary" icon="close" onClick={stop}>
            {t("runners.cancel")}
          </Button>
        )}
      </div>
      {showLog && (
        <pre ref={scroller} className={styles.log} tabIndex={-1}>
          {job.log.join("\n")}
        </pre>
      )}
    </div>
  );
}
