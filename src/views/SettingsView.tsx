import { useState, type ComponentType } from "react";
import { useTranslation } from "react-i18next";
import type { IconName } from "../components/Icon/Icon";
import { SideNav } from "../components/SideNav/SideNav";
import { AboutSection } from "./settings/AboutSection";
import { ArtworkSection } from "./settings/ArtworkSection";
import { EmulationSection } from "./settings/EmulationSection";
import { GeneralSection } from "./settings/GeneralSection";
import { LogsSection } from "./settings/LogsSection";
import { WindowsSection } from "./settings/WindowsSection";
import styles from "./SettingsView.module.scss";

export type SettingsSection = "general" | "windows" | "emulation" | "artwork" | "logs" | "about";

const SECTIONS: { value: SettingsSection; icon: IconName }[] = [
  { value: "general", icon: "settings" },
  { value: "windows", icon: "monitor" },
  { value: "emulation", icon: "console" },
  { value: "artwork", icon: "image" },
  { value: "logs", icon: "terminal" },
  { value: "about", icon: "info" },
];

const CONTENT: Record<SettingsSection, ComponentType> = {
  general: GeneralSection,
  windows: WindowsSection,
  emulation: EmulationSection,
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
      <aside className={styles.nav}>
        <h1 className={styles.title}>{t("settings.title")}</h1>
        <SideNav
          ariaLabel={t("settings.title")}
          items={SECTIONS.map((item) => ({ ...item, label: t(`settings.tabs.${item.value}`) }))}
          value={section}
          onChange={setSection}
        />
      </aside>

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
