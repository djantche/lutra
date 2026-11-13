import { useTranslation } from "react-i18next";
import { Select } from "../../../components/Select/Select";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import { TextField } from "../../../components/TextField/TextField";
import type { WineConfig, WineSettings } from "../../../lib/backend";
import styles from "./Tabs.module.scss";
import { VirtualDesktopField } from "./VirtualDesktopField";

interface WineTabProps {
  settings: WineSettings;
  onConfig: (patch: Partial<WineConfig>) => void;
}

/** Options that depend on the runner: Proton and Wine do not offer the same knobs. */
export function WineTab({ settings, onConfig }: WineTabProps) {
  const { t } = useTranslation();
  const { config } = settings;
  const isProton = settings.runnerKind === "proton";

  return (
    <SettingsGroup title={t("config.performance")}>
      <SettingsRow title={t("config.esync")} description={t("config.esyncHint")}>
        <Switch ariaLabel={t("config.esync")} checked={config.esync} onChange={(esync) => onConfig({ esync })} />
      </SettingsRow>
      <SettingsRow title={t("config.fsync")} description={t("config.fsyncHint")}>
        <Switch ariaLabel={t("config.fsync")} checked={config.fsync} onChange={(fsync) => onConfig({ fsync })} />
      </SettingsRow>

      {isProton && (
        <SettingsRow title={t("config.graphics")} description={t("config.graphicsHint")}>
          <div className={styles.control}>
            <Select
              ariaLabel={t("config.graphics")}
              value={config.wined3d ? "wined3d" : "dxvk"}
              options={[
                { value: "dxvk", label: t("config.graphicsDxvk") },
                { value: "wined3d", label: t("config.graphicsWined3d") },
              ]}
              onChange={(value) => onConfig({ wined3d: value === "wined3d" })}
            />
          </div>
        </SettingsRow>
      )}

      {!isProton && (
        <>
          <SettingsRow title={t("config.virtualDesktop")} description={t("config.virtualDesktopHint")}>
            <VirtualDesktopField
              value={config.virtualDesktop}
              onChange={(virtualDesktop) => onConfig({ virtualDesktop })}
            />
          </SettingsRow>
          <SettingsRow title={t("config.arch32")} description={t("config.arch32Hint")}>
            <Switch ariaLabel={t("config.arch32")} checked={config.arch32} onChange={(arch32) => onConfig({ arch32 })} />
          </SettingsRow>
        </>
      )}

      <SettingsRow stacked title={t("config.dllOverrides")} description={t("config.dllOverridesHint")}>
        <TextField
          value={config.dllOverrides}
          placeholder="d3d11=n,b;dinput8=n,b"
          spellCheck={false}
          onChange={(dllOverrides) => onConfig({ dllOverrides })}
        />
      </SettingsRow>
    </SettingsGroup>
  );
}
