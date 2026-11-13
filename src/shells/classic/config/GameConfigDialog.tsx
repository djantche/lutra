import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Dialog } from "../../../components/Dialog/Dialog";
import { GameArt } from "../../../components/GameArt/GameArt";
import { SideNav, type SideNavItem } from "../../../components/SideNav/SideNav";
import { Spinner } from "../../../components/Spinner/Spinner";
import { useEmulatorSettings } from "../../../hooks/useEmulatorSettings";
import { useGameSettings } from "../../../hooks/useGameSettings";
import { useLibraryStore } from "../../../state/library";
import { ArtworkTab } from "./artwork/ArtworkTab";
import { ControllerTab } from "./ControllerTab";
import { DetailsTab } from "./DetailsTab";
import { EmulatorTab } from "./EmulatorTab";
import { GameProfileTab } from "../../../views/emulation/profile/GameProfileTab";
import { GeneralTab } from "./GeneralTab";
import { LaunchTab } from "./LaunchTab";
import { LogsTab } from "./LogsTab";
import { PerformanceTab } from "./PerformanceTab";
import { WineTab } from "./WineTab";
import styles from "./GameConfigDialog.module.scss";

type Section =
  | "general"
  | "details"
  | "wine"
  | "performance"
  | "launch"
  | "video"
  | "controls"
  | "emulator"
  | "controller"
  | "artwork"
  | "logs";
const FORM_SECTIONS: Section[] = ["general", "details", "wine", "performance", "launch", "video", "controls", "emulator"];

/**
 * Settings window of a game. Every game can pick its artwork; games added by hand (Windows or
 * Linux) also get their details, performance and launch options (saved together with the Save
 * button), their controller profile and their log.
 */
export function GameConfigDialog({ gameId, onClose }: { gameId: string; onClose: () => void }) {
  const { t } = useTranslation();
  const game = useLibraryStore((state) => state.games.find((candidate) => candidate.id === gameId));
  const isManual = game?.source === "wine" || game?.source === "native";
  const isNative = game?.source === "native";
  const isEmulator = game?.source === "emulator";

  const [section, setSection] = useState<Section>(isManual || isEmulator ? "general" : "artwork");
  const { settings, details, setDetails, runners, tools, dirty, valid, saving, change, changeConfig, save } =
    useGameSettings(gameId, isManual, onClose);
  const emulator = useEmulatorSettings(gameId, isEmulator, onClose);

  if (!game) return null;

  const artwork: SideNavItem<Section> = { value: "artwork", label: t("config.tabArtwork"), icon: "image" };
  const sections: SideNavItem<Section>[] = isEmulator
    ? [
        { value: "general", label: t("config.tabGeneral"), icon: "info" },
        { value: "video", label: t("config.tabVideo"), icon: "monitor" },
        { value: "controls", label: t("config.tabControls"), icon: "gamepad" },
        { value: "emulator", label: t("config.tabEmulator"), icon: "console" },
        { value: "controller", label: t("config.tabController"), icon: "settings" },
        artwork,
        { value: "logs", label: t("config.tabLogs"), icon: "terminal" },
      ]
    : isManual
    ? [
        { value: "general", label: t("config.tabGeneral"), icon: "info" },
        { value: "details", label: t("config.tabDetails"), icon: "info" },
        ...(isNative ? [] : [{ value: "wine" as const, label: t("config.tabWine"), icon: "settings" as const }]),
        { value: "performance", label: t("config.tabPerformance"), icon: "settings" },
        { value: "launch", label: t("config.tabLaunch"), icon: "play" },
        { value: "controller", label: t("config.tabController"), icon: "gamepad" },
        artwork,
        { value: "logs", label: t("config.tabLogs"), icon: "terminal" },
      ]
    : [artwork];
  const formDirty = isEmulator ? emulator.dirty : dirty;
  const formValid = isEmulator ? true : valid;
  const formSaving = isEmulator ? emulator.saving : saving;
  const items = sections.map((item) => ({ ...item, badge: formDirty && FORM_SECTIONS.includes(item.value) }));

  const showsForm = FORM_SECTIONS.includes(section);

  return (
    <Dialog
      size="xl"
      title={game.title}
      description={t("config.subtitle", { source: t(`sources.${game.source}`) })}
      media={
        <div className={styles.cover}>
          <GameArt game={game} />
        </div>
      }
      onClose={onClose}
      busy={formSaving}
      footer={
        <>
          {formDirty && <span className={styles.unsaved}>{t("config.unsaved")}</span>}
          {(isManual || isEmulator) && showsForm ? (
            <>
              <Button variant="ghost" onClick={onClose} disabled={formSaving}>
                {t("config.close")}
              </Button>
              <Button
                variant="primary"
                disabled={!formDirty || !formValid || formSaving}
                onClick={isEmulator ? emulator.save : save}
              >
                {formSaving && <Spinner />}
                {t("config.save")}
              </Button>
            </>
          ) : (
            <Button variant="secondary" onClick={onClose}>
              {t("config.close")}
            </Button>
          )}
        </>
      }
    >
      <div className={styles.layout}>
        <SideNav ariaLabel={t("config.sections")} items={items} value={section} onChange={setSection} />

        <div className={styles.content}>
          {showsForm && !settings && !emulator.settings && <Spinner />}
          {emulator.settings && section === "general" && (
            <EmulatorTab
              settings={emulator.settings}
              cores={emulator.cores}
              onChange={emulator.change}
              onConfig={emulator.changeConfig}
            />
          )}
          {emulator.settings && (section === "video" || section === "controls" || section === "emulator") && (
            <GameProfileTab
              section={section}
              settings={emulator.settings}
              consoleProfile={emulator.consoleProfile}
              onConfig={emulator.changeConfig}
            />
          )}
          {settings && section === "general" && <GeneralTab settings={settings} runners={runners} onChange={change} />}
          {details && section === "details" && (
            <DetailsTab title={settings?.title ?? game.title} details={details} onChange={setDetails} />
          )}
          {settings && section === "wine" && <WineTab settings={settings} onConfig={changeConfig} />}
          {settings && section === "performance" && (
            <PerformanceTab config={settings.config} runnerKind={settings.runnerKind} onConfig={changeConfig} />
          )}
          {settings && section === "launch" && (
            <LaunchTab config={settings.config} tools={tools} native={isNative} onConfig={changeConfig} />
          )}
          {section === "controller" && <ControllerTab gameId={game.id} />}
          {section === "artwork" && <ArtworkTab game={game} />}
          {section === "logs" && <LogsTab gameId={game.id} />}
        </div>
      </div>
    </Dialog>
  );
}
