import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Select } from "../../components/Select/Select";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import {
  defaultRunner,
  listRunners,
  openPrefixesFolder,
  setDefaultRunner,
  wineTools,
  type Runner,
  type Tools,
} from "../../lib/backend";
import { reportError } from "../../state/toasts";
import styles from "./WindowsSection.module.scss";
import { ToolsStatus } from "./ToolsStatus";

const AUTOMATIC = "";

export function WindowsSection() {
  const { t } = useTranslation();
  const [runners, setRunners] = useState<Runner[]>([]);
  const [chosen, setChosen] = useState(AUTOMATIC);
  const [tools, setTools] = useState<Tools | null>(null);

  useEffect(() => {
    listRunners()
      .then(setRunners)
      .catch(() => setRunners([]));
    defaultRunner()
      .then((path) => setChosen(path ?? AUTOMATIC))
      .catch(() => setChosen(AUTOMATIC));
    wineTools()
      .then(setTools)
      .catch(() => setTools(null));
  }, []);

  const choose = (path: string) => {
    setChosen(path);
    setDefaultRunner(path === AUTOMATIC ? null : path).catch((error) => reportError(t("errors.settings"), error));
  };

  return (
    <>
    <SettingsGroup title={t("settings.groupLaunch")}>
      <SettingsRow title={t("settings.defaultRunner")} description={t("settings.defaultRunnerHint")}>
        <div className={styles.select}>
          <Select
            ariaLabel={t("settings.defaultRunner")}
            value={chosen}
            options={[
              { value: AUTOMATIC, label: t("settings.runnerAutomatic") },
              ...runners.map((runner) => ({ value: runner.path, label: runner.label })),
            ]}
            onChange={choose}
          />
        </div>
      </SettingsRow>

      <SettingsRow title={t("settings.prefixes")} description={t("settings.prefixesHint")}>
        <Button
          variant="secondary"
          icon="folder"
          onClick={() => openPrefixesFolder().catch((error) => reportError(t("errors.folder"), error))}
        >
          {t("settings.openFolder")}
        </Button>
      </SettingsRow>

    </SettingsGroup>

      {tools && (
        <SettingsGroup title={t("settings.tools")}>
          <SettingsRow stacked title={t("settings.toolsFound")} description={t("settings.toolsHint")}>
            <ToolsStatus tools={tools} />
          </SettingsRow>
        </SettingsGroup>
      )}
    </>
  );
}
