import { useState, type ComponentType } from "react";
import { useTranslation } from "react-i18next";
import type { IconName } from "../components/Icon/Icon";
import { TabBar } from "../components/TabBar/TabBar";
import { AboutSection } from "./settings/AboutSection";
import { AccessibilitySection } from "./settings/AccessibilitySection";
import { AudioSection } from "./settings/AudioSection";
import { ArtworkSection } from "./settings/ArtworkSection";
import { GeneralSection } from "./settings/GeneralSection";
import { LogsSection } from "./settings/LogsSection";
import { RunnersSection } from "./settings/RunnersSection";
import { WindowsSection } from "./settings/WindowsSection";
import styles from "./SettingsView.module.scss";

export type SettingsSection = "general" | "audio" | "accessibility" | "windows" | "runners" | "artwork" | "logs" | "about";

const SECTIONS: { value: SettingsSection; icon: IconName }[] = [
  { value: "general", icon: "settings" },
  { value: "audio", icon: "volume" },
  { value: "accessibility", icon: "accessibility" },
  { value: "windows", icon: "monitor" },
  { value: "runners", icon: "download" },
  { value: "artwork", icon: "image" },
  { value: "logs", icon: "terminal" },
  { value: "about", icon: "info" },
];

const CONTENT: Record<SettingsSection, ComponentType> = {
  general: GeneralSection,
  audio: AudioSection,
  accessibility: AccessibilitySection,
  windows: WindowsSection,
  runners: RunnersSection,
  artwork: ArtworkSection,
  logs: LogsSection,
  about: AboutSection,
};

/**
 * Settings page, shared by both interface modes: categories on the side, one at a time on the
 * right. Sized in em so each shell can scale it.
 */
export function SettingsView({ section: initial = "general" }: { section?: SettingsSection }) {
  const { t } = useTranslation();
  const [section, setSection] = useState<SettingsSection>(initial);
  const Content = CONTENT[section];

  return (
    <div className={styles.settings}>
      <header className={styles.top}>
        <h1 className={styles.title}>{t("settings.title")}</h1>
        <TabBar
          ariaLabel={t("settings.title")}
          items={SECTIONS.map((item) => ({ ...item, label: t(`settings.tabs.${item.value}`) }))}
          value={section}
          onChange={setSection}
        />
      </header>

      <section key={section} className={styles.panel}>
        <header className={styles.panelHeader}>
          <h2>{t(`settings.tabs.${section}`)}</h2>
          <p>{t(`settings.tabsHint.${section}`)}</p>
        </header>
        <Content />
      </section>
    </div>
  );
}
