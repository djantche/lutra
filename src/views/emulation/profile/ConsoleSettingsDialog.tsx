import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Dialog } from "../../../components/Dialog/Dialog";
import { SideNav } from "../../../components/SideNav/SideNav";
import { Spinner } from "../../../components/Spinner/Spinner";
import { SystemIcon } from "../../../components/SystemIcon/SystemIcon";
import { emulationProfile, saveEmulationProfile, type EmulationProfile } from "../../../lib/backend";
import { platformOf } from "../../../lib/platforms";
import { reportError } from "../../../state/toasts";
import { ProfileEditor, type ProfileSection } from "./ProfileEditor";
import { PROFILE_SECTIONS } from "./sections";
import styles from "./Profile.module.scss";

/** The settings every game of a console shares, saved as they change. */
export function ConsoleSettingsDialog({ system, icon, onClose }: { system: string; icon: string | null; onClose: () => void }) {
  const { t } = useTranslation();
  const platform = platformOf(system);
  const [section, setSection] = useState<ProfileSection>("video");
  const [profile, setProfile] = useState<EmulationProfile | null>(null);

  useEffect(() => {
    emulationProfile(system)
      .then(setProfile)
      .catch((error) => reportError(t("errors.settings"), error));
  }, [system, t]);

  const change = (next: EmulationProfile) => {
    setProfile(next);
    saveEmulationProfile(system, next).catch((error) => reportError(t("errors.save"), error));
  };

  return (
    <Dialog
      size="xl"
      title={t("emulation.profile.consoleTitle", { name: platform.name })}
      description={t("emulation.profile.consoleHint")}
      media={<SystemIcon platform={platform} icon={icon} className={styles.icon} />}
      onClose={onClose}
      footer={
        <Button variant="secondary" onClick={onClose}>
          {t("config.close")}
        </Button>
      }
    >
      <div className={styles.layout}>
        <SideNav
          ariaLabel={t("config.sections")}
          items={PROFILE_SECTIONS.map((item) => ({ ...item, label: t(`emulation.profile.section.${item.value}`) }))}
          value={section}
          onChange={setSection}
        />
        <div className={styles.content}>
          {profile ? <ProfileEditor system={system} section={section} profile={profile} onChange={change} /> : <Spinner />}
        </div>
      </div>
    </Dialog>
  );
}
