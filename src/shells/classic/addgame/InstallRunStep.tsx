import { useTranslation } from "react-i18next";
import { FilePicker } from "../../../components/FilePicker/FilePicker";
import { FormField } from "../../../components/FormField/FormField";
import { Select } from "../../../components/Select/Select";
import { Spinner } from "../../../components/Spinner/Spinner";
import { pickProgram, type InstallFinished } from "../../../lib/backend";
import { reportError } from "../../../state/toasts";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

const megabytes = (bytes: number) => `${Math.max(1, Math.round(bytes / (1024 * 1024)))} MB`;

interface InstallRunStepProps {
  /** Null while the installer runs. */
  finished: InstallFinished | null;
  exePath: string;
  onExe: (path: string) => void;
}

/**
 * The installer runs (its real process), then the programs it added are offered so the game's own
 * executable can be chosen.
 */
export function InstallRunStep({ finished, exePath, onExe }: InstallRunStepProps) {
  const { t } = useTranslation();

  if (!finished) {
    return (
      <>
        <StepHeading icon="download" title={t("addGame.steps.install.title")} hint={t("addGame.steps.install.hint")} />
        <div className={styles.chosen}>
          <Spinner />
          <div className={styles.chosenText}>
            <strong>{t("install.running")}</strong>
            <small>{t("install.runningHint")}</small>
          </div>
        </div>
      </>
    );
  }

  return (
    <>
      <StepHeading icon="download" title={t("addGame.steps.install.doneTitle")} hint={t("addGame.steps.install.doneHint")} />

      {!finished.success && (
        <p className={styles.warning}>
          {t("install.failed")}
          {finished.message && <code className={styles.log}>{finished.message}</code>}
        </p>
      )}

      {finished.candidates.length > 0 ? (
        <FormField label={t("install.program")} hint={t("install.programHint")}>
          <Select
            ariaLabel={t("install.program")}
            value={exePath}
            onChange={onExe}
            autofocus
            options={finished.candidates.map((candidate) => ({
              value: candidate.path,
              label: candidate.shown,
              hint: megabytes(candidate.size),
            }))}
          />
        </FormField>
      ) : (
        <FormField label={t("install.program")} hint={t("install.noneFound")}>
          <FilePicker
            autofocus
            value={exePath}
            placeholder={t("addGame.browse")}
            onPick={() =>
              pickProgram()
                .then((picked) => picked && onExe(picked))
                .catch((error) => reportError(t("errors.pick"), error))
            }
          />
        </FormField>
      )}
    </>
  );
}
