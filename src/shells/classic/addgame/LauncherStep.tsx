import { useTranslation } from "react-i18next";
import { ChoiceCards } from "../../../components/ChoiceCards/ChoiceCards";
import { FormField } from "../../../components/FormField/FormField";
import { NEW_PREFIX, PrefixPicker } from "../../../components/PrefixPicker/PrefixPicker";
import { InstallLatest } from "../../../components/RunnerInstall/InstallLatest";
import { Switch } from "../../../components/Switch/Switch";
import { TextField } from "../../../components/TextField/TextField";
import { PREFIX_NAME_MAX } from "../../../lib/prefixes";
import type { Runner } from "../../../lib/backend";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

interface LauncherStepProps {
  /** Null while the runners are being looked for. */
  runners: Runner[] | null;
  runnerPath: string;
  onRunner: (path: string) => void;
  /** The runner chosen in the settings, if it is still installed. */
  defaultPath: string | null;
  makeDefault: boolean;
  onMakeDefault: (on: boolean) => void;
  prefixPath: string;
  onPrefix: (path: string) => void;
  /** What a new prefix will be called (the game's name until the user types another). */
  prefixName: string;
  onPrefixName: (name: string) => void;
  installing: boolean;
}

/** Which Wine or Proton runs the game, as what is already set up; and in which prefix. */
export function LauncherStep({
  runners,
  runnerPath,
  onRunner,
  defaultPath,
  makeDefault,
  onMakeDefault,
  prefixPath,
  onPrefix,
  prefixName,
  onPrefixName,
  installing,
}: LauncherStepProps) {
  const { t } = useTranslation();
  const runner = runners?.find((candidate) => candidate.path === runnerPath);

  return (
    <>
      <StepHeading icon="settings" title={t("addGame.steps.launcher.title")} hint={t("addGame.steps.launcher.hint")} />

      {runners === null ? null : runners.length === 0 ? (
        <>
          <p className={styles.warning}>{t("addGame.noRunner")}</p>
          <InstallLatest />
        </>
      ) : (
        <>
          <ChoiceCards
            layout="list"
            autofocus
            ariaLabel={t("addGame.runner")}
            value={runnerPath}
            onChange={onRunner}
            options={runners.map((candidate) => ({
              value: candidate.path,
              icon: candidate.kind === "proton" ? "play" : "console",
              title: candidate.label,
              description: candidate.kind === "proton" ? t("addGame.runnerProton") : t("addGame.runnerWine"),
              badge: candidate.path === defaultPath ? t("addGame.runnerDefault") : undefined,
            }))}
          />

          {runner && runner.path !== defaultPath && (
            <div className={styles.defaultRow}>
              <div>
                <strong>{t("addGame.makeDefault")}</strong>
                <small>{t("addGame.makeDefaultHint", { name: runner.label })}</small>
              </div>
              <Switch ariaLabel={t("addGame.makeDefault")} checked={makeDefault} onChange={onMakeDefault} />
            </div>
          )}

          {runner && (
            <FormField label={t("config.prefix")}>
              <PrefixPicker allowNew ariaLabel={t("config.prefix")} runnerKind={runner.kind} value={prefixPath} onChange={onPrefix} />
            </FormField>
          )}

          {runner && prefixPath === NEW_PREFIX && (
            <FormField label={t("addGame.prefixName")} hint={t("addGame.prefixNameHint")}>
              <TextField
                value={prefixName}
                onChange={onPrefixName}
                maxLength={PREFIX_NAME_MAX}
                placeholder={t("addGame.namePlaceholder")}
                icon="folder"
              />
            </FormField>
          )}
          <p className={styles.note}>{installing ? t("install.optimized") : t("addGame.optimized")}</p>
        </>
      )}
    </>
  );
}

