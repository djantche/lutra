import { useTranslation } from "react-i18next";
import { Switch } from "../../../components/Switch/Switch";
import type { EmulationProfile, EmulatorConfig, EmulatorSettings } from "../../../lib/backend";
import { platformOf } from "../../../lib/platforms";
import { ProfileEditor, type ProfileSection } from "./ProfileEditor";
import styles from "./Profile.module.scss";

interface GameProfileTabProps {
  section: ProfileSection;
  settings: EmulatorSettings;
  /** What the game uses while it has no settings of its own. */
  consoleProfile: EmulationProfile;
  onConfig: (patch: Partial<EmulatorConfig>) => void;
}

/**
 * A game's picture, controls or emulator options: its console's, or its own when the user keeps
 * settings for this game only (they start as a copy of the console's).
 */
export function GameProfileTab({ section, settings, consoleProfile, onConfig }: GameProfileTabProps) {
  const { t } = useTranslation();
  const own = settings.config.profile;
  const consoleName = platformOf(settings.config.system).name;

  return (
    <>
      <div className={styles.scope}>
        <div>
          <strong>{t("emulation.profile.own")}</strong>
          <small>{own ? t("emulation.profile.ownOn") : t("emulation.profile.ownOff", { name: consoleName })}</small>
        </div>
        <Switch
          ariaLabel={t("emulation.profile.own")}
          checked={own !== null}
          onChange={(on) => onConfig({ profile: on ? structuredClone(consoleProfile) : null })}
        />
      </div>
      <div inert={own === null} className={own === null ? styles.locked : undefined}>
        <ProfileEditor
          system={settings.config.system}
          section={section}
          profile={own ?? consoleProfile}
          onChange={(profile) => onConfig({ profile })}
        />
      </div>
    </>
  );
}
