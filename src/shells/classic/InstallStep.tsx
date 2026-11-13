import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FilePicker } from "../../components/FilePicker/FilePicker";
import { FormField } from "../../components/FormField/FormField";
import { GameDetailsFields } from "../../components/GameDetailsFields/GameDetailsFields";
import {
  detailsFromMeta,
  EMPTY_DETAILS,
  type DetailsForm,
} from "../../components/GameDetailsFields/details";
import { GameMatches } from "../../components/GameMatches/GameMatches";
import { Select } from "../../components/Select/Select";
import { Spinner } from "../../components/Spinner/Spinner";
import { useGameLookup } from "../../hooks/useGameLookup";
import {
  cancelInstall,
  onInstallFinished,
  pickProgram,
  type GameHit,
  type InstallFinished,
} from "../../lib/backend";
import { reportError } from "../../state/toasts";
import styles from "./AddGameDialog.module.scss";

/** What the user settled on once the installer closed: the game to add. */
export interface InstallChoice {
  title: string;
  exePath: string;
  prefixPath: string;
  details: DetailsForm;
}

export interface InstallStepProps {
  installId: string;
  title: string;
  /** The prefix chosen or created for the installation. */
  prefixPath: string;
  /** Reports the current choice, or null while there is nothing to add yet. */
  onChoice: (choice: InstallChoice | null) => void;
  /** Back to the form (the installer was cancelled). */
  onBack: () => void;
  busy: boolean;
}

const megabytes = (bytes: number) => `${Math.max(1, Math.round(bytes / (1024 * 1024)))} MB`;

/**
 * The second half of installing a game: the installer is running (real: its process), then the
 * programs it added are offered so the game's own executable can be chosen.
 */
export function InstallStep({ installId, title, prefixPath, onChoice, onBack, busy }: InstallStepProps) {
  const { t } = useTranslation();
  const [finished, setFinished] = useState<InstallFinished | null>(null);
  const [exePath, setExePath] = useState("");
  const [name, setName] = useState(title);
  const [details, setDetails] = useState<DetailsForm>(EMPTY_DETAILS);
  const [matchId, setMatchId] = useState<number | null>(null);
  const lookup = useGameLookup(finished ? name : "");

  useEffect(() => {
    const stop = onInstallFinished((event) => {
      if (event.installId !== installId) return;
      setFinished(event);
      // The likeliest program is already selected; the user confirms or changes it.
      setExePath(event.candidates[0]?.path ?? "");
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [installId]);

  useEffect(() => {
    onChoice(
      finished !== null && exePath !== ""
        ? { title: name.trim() || title, exePath, prefixPath: finished.prefixPath || prefixPath, details }
        : null,
    );
    // The parent's callback changes on every render; only what is chosen matters here.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [finished, exePath, name, details]);

  const useMatch = (hit: GameHit) => {
    setMatchId(hit.id);
    setName(hit.name);
    setDetails(detailsFromMeta(hit.meta));
  };

  if (!finished) {
    return (
      <div className={styles.installing}>
        <p className={styles.status}>
          <Spinner /> {t("install.running")}
        </p>
        <p className={styles.note}>{t("install.runningHint")}</p>
        <button
          type="button"
          data-focusable
          className={styles.link}
          disabled={busy}
          onClick={() => {
            cancelInstall(installId).catch((error) => reportError(t("install.cancelFailed"), error));
            onBack();
          }}
        >
          {t("install.cancel")}
        </button>
      </div>
    );
  }

  return (
    <>
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
            onChange={setExePath}
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
            value={exePath}
            placeholder={t("addGame.browse")}
            onPick={() =>
              pickProgram()
                .then((picked) => picked && setExePath(picked))
                .catch((error) => reportError(t("errors.pick"), error))
            }
          />
        </FormField>
      )}

      <div className={styles.section}>
        <span className={styles.sectionTitle}>{t("details.fromGameDb")}</span>
        <GameMatches {...lookup} selected={matchId} onSelect={useMatch} />
      </div>
      <GameDetailsFields value={details} onChange={setDetails} />
    </>
  );
}
