import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { FilePicker } from "../../../components/FilePicker/FilePicker";
import { KeyValueList } from "../../../components/KeyValueList/KeyValueList";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import { TextField } from "../../../components/TextField/TextField";
import { pickFolder, type Tools, type WineConfig } from "../../../lib/backend";
import { reportError } from "../../../state/toasts";
import styles from "./Tabs.module.scss";

interface LaunchTabProps {
  config: WineConfig;
  tools: Tools | null;
  /** A Linux program: no Wine, so no Wine debug output. */
  native: boolean;
  onConfig: (patch: Partial<WineConfig>) => void;
}

export function LaunchTab({ config, tools, native, onConfig }: LaunchTabProps) {
  const { t } = useTranslation();
  const hint = (installed: boolean, text: string) => (tools && !installed ? t("config.notInstalled") : text);

  return (
    <>
      <SettingsGroup title={t("config.launch")}>
        <SettingsRow stacked title={t("config.args")} description={t("config.argsHint")}>
          <TextField
            value={config.args}
            placeholder="-windowed -nointro"
            spellCheck={false}
            onChange={(args) => onConfig({ args })}
          />
        </SettingsRow>

        <SettingsRow stacked title={t("config.workingDir")} description={t("config.workingDirHint")}>
          <div className={styles.inline}>
            <FilePicker
              value={config.workingDir ?? ""}
              placeholder={t("config.workingDirDefault")}
              onPick={() =>
                pickFolder()
                  .then((workingDir) => workingDir && onConfig({ workingDir }))
                  .catch((error) => reportError(t("errors.pick"), error))
              }
            />
            {config.workingDir && (
              <Button variant="ghost" onClick={() => onConfig({ workingDir: null })}>
                {t("common.reset")}
              </Button>
            )}
          </div>
        </SettingsRow>

        <SettingsRow stacked title={t("config.env")} description={t("config.envHint")}>
          <KeyValueList
            items={config.env}
            onChange={(env) => onConfig({ env })}
            keyPlaceholder={t("config.envName")}
            valuePlaceholder={t("config.envValue")}
            addLabel={t("config.envAdd")}
          />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("config.tools")}>
        <SettingsRow title="GameMode" description={hint(Boolean(tools?.gamemode), t("config.gamemodeHint"))}>
          <Switch
            ariaLabel="GameMode"
            checked={config.gamemode}
            disabled={tools !== null && !tools.gamemode}
            onChange={(gamemode) => onConfig({ gamemode })}
          />
        </SettingsRow>
        <SettingsRow title="MangoHud" description={hint(Boolean(tools?.mangohud), t("config.mangohudHint"))}>
          <Switch
            ariaLabel="MangoHud"
            checked={config.mangohud}
            disabled={tools !== null && !tools.mangohud}
            onChange={(mangohud) => onConfig({ mangohud })}
          />
        </SettingsRow>
        {!native && (
          <SettingsRow title={t("config.debug")} description={t("config.debugHint")}>
            <Switch ariaLabel={t("config.debug")} checked={config.debug} onChange={(debug) => onConfig({ debug })} />
          </SettingsRow>
        )}
      </SettingsGroup>
    </>
  );
}
