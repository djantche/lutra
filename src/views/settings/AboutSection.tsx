import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { hasBackend } from "../../lib/backend";
import styles from "./AboutSection.module.scss";

/** What Lutra is, which version runs, and under which licence. */
export function AboutSection() {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    if (!hasBackend()) {
      setVersion("dev");
      return;
    }
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);

  return (
    <>
      <div className={styles.card}>
        <img src="/lutra.svg" alt="" width={64} height={64} />
        <div>
          <strong>Lutra</strong>
          <span>{version ? t("settings.version", { version }) : null}</span>
          <p>{t("settings.aboutHint")}</p>
        </div>
      </div>
      <SettingsGroup title={t("settings.license")}>
        <SettingsRow title="GPL-3.0-or-later" description={t("settings.licenseHint")} />
      </SettingsGroup>
    </>
  );
}
