import { useTranslation } from "react-i18next";
import { ChoiceCards } from "../../../components/ChoiceCards/ChoiceCards";
import { FilePicker } from "../../../components/FilePicker/FilePicker";
import { FormField } from "../../../components/FormField/FormField";
import { StepHeading } from "./StepHeading";

export type Kind = "windows" | "linux" | "install";

interface SourceStepProps {
  kind: Kind;
  onKind: (kind: Kind) => void;
  exePath: string;
  onBrowse: () => void;
}

/** First step: what kind of game it is, and its program (or its installer). */
export function SourceStep({ kind, onKind, exePath, onBrowse }: SourceStepProps) {
  const { t } = useTranslation();
  const installing = kind === "install";

  return (
    <>
      <StepHeading icon="gamepad" title={t("addGame.steps.source.title")} hint={t("addGame.steps.source.hint")} />

      <ChoiceCards
        ariaLabel={t("addGame.kind")}
        autofocus
        value={kind}
        onChange={onKind}
        options={[
          { value: "windows", icon: "monitor", title: t("addGame.kindWindows"), description: t("addGame.kindWindowsHint") },
          { value: "linux", icon: "terminal", title: t("addGame.kindLinux"), description: t("addGame.kindLinuxHint") },
          { value: "install", icon: "download", title: t("addGame.kindInstall"), description: t("addGame.kindInstallHint") },
        ]}
      />

      <FormField
        label={installing ? t("install.installer") : t("addGame.program")}
        hint={kind === "linux" ? t("addGame.linuxHint") : installing ? t("install.installerHint") : undefined}
      >
        <FilePicker value={exePath} placeholder={t("addGame.browse")} onPick={onBrowse} />
      </FormField>
    </>
  );
}
