import { useTranslation } from "react-i18next";
import { FormField } from "../../../components/FormField/FormField";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import { TextField } from "../../../components/TextField/TextField";
import type { CoreChoice, EmulatorConfig, EmulatorSettings } from "../../../lib/backend";
import { platformOf } from "../../../lib/platforms";

interface EmulatorTabProps {
  settings: EmulatorSettings;
  cores: CoreChoice[];
  onChange: (patch: Partial<EmulatorSettings>) => void;
  onConfig: (patch: Partial<EmulatorConfig>) => void;
}

/** An emulated game: its name, its console, the emulator Lutra runs it with, and how it starts. */
export function EmulatorTab({ settings, cores, onChange, onConfig }: EmulatorTabProps) {
  const { t } = useTranslation();
  const { config } = settings;
  const used = cores[0];

  return (
    <>
      <FormField label={t("addGame.name")}>
        <TextField value={settings.title} onChange={(title) => onChange({ title })} />
      </FormField>

      <SettingsGroup title={t("emulation.game.system")}>
        <SettingsRow title={platformOf(config.system).name} description={settings.romPath} />
        <SettingsRow
          title={used ? used.name : t("emulation.noCore")}
          description={used ? t("emulation.coreByLutra") : t("emulation.noCoreAuto")}
        />
      </SettingsGroup>

      <SettingsGroup title={t("emulation.game.start")}>
        <SettingsRow title={t("emulation.game.fullscreen")} description={t("emulation.game.fullscreenHint")}>
          <Switch
            ariaLabel={t("emulation.game.fullscreen")}
            checked={config.fullscreen}
            onChange={(fullscreen) => onConfig({ fullscreen })}
          />
        </SettingsRow>
        <SettingsRow title={t("emulation.game.resume")} description={t("emulation.game.resumeHint")}>
          <Switch
            ariaLabel={t("emulation.game.resume")}
            checked={config.autoResume}
            onChange={(autoResume) => onConfig({ autoResume })}
          />
        </SettingsRow>
      </SettingsGroup>
    </>
  );
}
