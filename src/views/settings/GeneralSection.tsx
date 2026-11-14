import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Switch } from "../../components/Switch/Switch";
import { setTraySettings, traySettings, type TraySettings } from "../../lib/backend";
import { reportError } from "../../state/toasts";
import { Select } from "../../components/Select/Select";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { LANGUAGES, setLanguage } from "../../i18n";
import { setTheme, theme, type ThemeChoice } from "../../lib/theme";
import { useUi } from "../../state/ui";
import styles from "./GeneralSection.module.scss";

export function GeneralSection() {
  const { t, i18n } = useTranslation();
  const { defaultMode, setDefaultMode } = useUi();
  const [tray, setTray] = useState<TraySettings | null>(null);
  const [appearance, setAppearance] = useState<ThemeChoice>(theme);

  useEffect(() => {
    traySettings()
      .then(setTray)
      .catch(() => setTray(null));
  }, []);

  const changeTray = (patch: Partial<TraySettings>) => {
    if (!tray) return;
    const next = { ...tray, ...patch };
    setTray(next);
    setTraySettings(next).catch((error) => reportError(t("errors.save"), error));
  };

  return (
    <>
    <SettingsGroup title={t("settings.groupInterface")}>
      <SettingsRow title={t("settings.language")} description={t("settings.languageHint")}>
        <div className={styles.language}>
          <Select
            ariaLabel={t("settings.language")}
            options={LANGUAGES.map((language) => ({ value: language.code as string, label: language.label }))}
            value={i18n.resolvedLanguage ?? "en"}
            onChange={setLanguage}
            autofocus
          />
        </div>
      </SettingsRow>

      <SettingsRow title={t("settings.theme")} description={t("settings.themeHint")}>
        <SegmentedControl
          ariaLabel={t("settings.theme")}
          segments={[
            { value: "system", icon: "monitor", label: t("settings.themeSystem") },
            { value: "light", icon: "sun", label: t("settings.themeLight") },
            { value: "dark", icon: "moon", label: t("settings.themeDark") },
          ]}
          value={appearance}
          onChange={(choice) => {
            setAppearance(choice);
            setTheme(choice);
          }}
        />
      </SettingsRow>

      <SettingsRow title={t("settings.defaultMode")} description={t("settings.defaultModeHint")}>
        <SegmentedControl
          ariaLabel={t("settings.defaultMode")}
          segments={[
            { value: "classic", label: t("settings.modeClassic") },
            { value: "bigpicture", label: t("settings.modeBigPicture") },
          ]}
          value={defaultMode}
          onChange={setDefaultMode}
        />
      </SettingsRow>
    </SettingsGroup>

    {tray && (
      <SettingsGroup title={t("settings.groupTray")}>
        {!tray.available && <SettingsRow title={t("settings.trayMissing")} description={t("settings.trayMissingHint")} />}
        <SettingsRow title={t("settings.trayClose")} description={t("settings.trayCloseHint")}>
          <Switch
            ariaLabel={t("settings.trayClose")}
            checked={tray.closeToTray}
            disabled={!tray.available}
            onChange={(closeToTray) => changeTray({ closeToTray })}
          />
        </SettingsRow>
        <SettingsRow title={t("settings.trayGames")} description={t("settings.trayGamesHint")}>
          <Switch
            ariaLabel={t("settings.trayGames")}
            checked={tray.hideDuringGames}
            disabled={!tray.available}
            onChange={(hideDuringGames) => changeTray({ hideDuringGames })}
          />
        </SettingsRow>
        <SettingsRow title={t("settings.trayStart")} description={t("settings.trayStartHint")}>
          <Switch
            ariaLabel={t("settings.trayStart")}
            checked={tray.startInTray}
            disabled={!tray.available}
            onChange={(startInTray) => changeTray({ startInTray })}
          />
        </SettingsRow>
      </SettingsGroup>
    )}
    </>
  );
}
