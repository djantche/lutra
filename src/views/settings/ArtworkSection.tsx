import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../components/Switch/Switch";
import { artSettings, setArtSettings, type ArtSettings } from "../../lib/backend";
import { SteamGridDbKey } from "./SteamGridDbKey";

export function ArtworkSection() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<ArtSettings | null>(null);

  useEffect(() => {
    artSettings()
      .then(setSettings)
      .catch((error) => console.error("Reading artwork settings failed:", error));
  }, []);

  const update = (steamSource: ArtSettings["steamSource"], fallback: boolean) => {
    setArtSettings(steamSource, fallback)
      .then(setSettings)
      .catch((error) => console.error("Saving artwork settings failed:", error));
  };

  if (!settings) return null;

  return (
    <>
    <SettingsGroup title="Steam">
      <SettingsRow title={t("settings.steamArt")} description={t("settings.steamArtHint")}>
        <SegmentedControl
          ariaLabel={t("settings.steamArt")}
          segments={[
            { value: "official", label: t("settings.artSteam") },
            { value: "custom", label: t("settings.artCustom") },
          ]}
          value={settings.steamSource}
          onChange={(source) => update(source, settings.fallback)}
        />
      </SettingsRow>

      {settings.steamSource === "custom" && (
        <SettingsRow title={t("settings.fallback")} description={t("settings.fallbackHint")}>
          <Switch
            ariaLabel={t("settings.fallback")}
            checked={settings.fallback}
            onChange={(fallback) => update(settings.steamSource, fallback)}
          />
        </SettingsRow>
      )}

    </SettingsGroup>

    <SettingsGroup title="SteamGridDB">
      <SettingsRow stacked title={t("settings.steamGridDbKey")} description={t("settings.steamGridDbHint")}>
        <SteamGridDbKey
          connected={settings.hasKey}
          onChange={(hasKey) => setSettings((current) => current && { ...current, hasKey })}
        />
      </SettingsRow>
    </SettingsGroup>
    </>
  );
}
