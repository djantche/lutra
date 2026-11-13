import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FilePicker } from "../../../components/FilePicker/FilePicker";
import { FormField } from "../../../components/FormField/FormField";
import { PrefixPicker } from "../../../components/PrefixPicker/PrefixPicker";
import { Select } from "../../../components/Select/Select";
import { TextField } from "../../../components/TextField/TextField";
import {
  checkPrefix,
  pickLinuxProgram,
  pickProgram,
  type Runner,
  type WineSettings,
} from "../../../lib/backend";
import { reportError } from "../../../state/toasts";

interface GeneralTabProps {
  settings: WineSettings;
  runners: Runner[];
  onChange: (patch: Partial<WineSettings>) => void;
}

export function GeneralTab({ settings, runners, onChange }: GeneralTabProps) {
  const { t } = useTranslation();
  const [runnerError, setRunnerError] = useState<string | null>(null);
  const native = settings.runnerKind === "native";

  // The runner a game was saved with may no longer be detected: keep it selectable.
  const options = runners.some((runner) => runner.path === settings.runnerPath)
    ? runners
    : [
        ...runners,
        {
          kind: settings.runnerKind as Runner["kind"],
          label: settings.runnerPath.split("/").slice(-2, -1)[0] ?? "?",
          path: settings.runnerPath,
        },
      ];

  // The prefix decides which runners can be chosen: Wine and Proton prefixes are not interchangeable.
  const chooseRunner = (runnerPath: string) => {
    const runner = options.find((candidate) => candidate.path === runnerPath);
    if (!runner) return;
    setRunnerError(null);
    checkPrefix(settings.prefixPath, runner.kind).then(
      () => onChange({ runnerPath, runnerKind: runner.kind }),
      (reason) => setRunnerError(String(reason)),
    );
  };

  return (
    <>
      <FormField label={t("addGame.name")}>
        <TextField value={settings.title} onChange={(title) => onChange({ title })} />
      </FormField>

      <FormField label={t("addGame.program")} hint={native ? t("addGame.linuxHint") : undefined}>
        <FilePicker
          value={settings.exePath}
          placeholder={t("addGame.browse")}
          onPick={() =>
            (native ? pickLinuxProgram() : pickProgram())
              .then((exePath) => exePath && onChange({ exePath }))
              .catch((error) => reportError(t("errors.pick"), error))
          }
        />
      </FormField>

      {!native && (
        <>
          <FormField label={t("addGame.runner")} hint={t("config.runnerHint")} error={runnerError}>
            <Select
              ariaLabel={t("addGame.runner")}
              value={settings.runnerPath}
              options={options.map((runner) => ({
                value: runner.path,
                label: runner.label,
                hint: runner.kind === "proton" ? "Proton" : "Wine",
              }))}
              onChange={chooseRunner}
            />
          </FormField>

          <FormField
            label={t("config.prefix")}
            hint={settings.prefixExists ? t("config.prefixReady") : t("config.prefixLater")}
          >
            <PrefixPicker
              ariaLabel={t("config.prefix")}
              runnerKind={settings.runnerKind as Runner["kind"]}
              value={settings.prefixPath}
              onChange={(prefixPath) => onChange({ prefixPath })}
            />
          </FormField>
        </>
      )}
    </>
  );
}
