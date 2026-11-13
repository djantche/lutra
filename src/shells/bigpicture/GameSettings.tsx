import { useEffect, useState, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { GameArt } from "../../components/GameArt/GameArt";
import { Icon, type IconName } from "../../components/Icon/Icon";
import { Spinner } from "../../components/Spinner/Spinner";
import { TabBar } from "../../components/TabBar/TabBar";
import { useGameSettings } from "../../hooks/useGameSettings";
import { hoverFocus } from "../../input/hoverFocus";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import type { Game } from "../../types/game";
import { ArtworkTab } from "../classic/config/artwork/ArtworkTab";
import { ControllerTab } from "../classic/config/ControllerTab";
import { DetailsTab } from "../classic/config/DetailsTab";
import { GeneralTab } from "../classic/config/GeneralTab";
import { LaunchTab } from "../classic/config/LaunchTab";
import { LogsTab } from "../classic/config/LogsTab";
import { PerformanceTab } from "../classic/config/PerformanceTab";
import { WineTab } from "../classic/config/WineTab";
import styles from "./GameSettings.module.scss";

type Section =
  | "general"
  | "details"
  | "wine"
  | "performance"
  | "launch"
  | "controller"
  | "artwork"
  | "logs";

/** How long after the last change the settings are written: one write for a burst of edits. */
const AUTOSAVE_MS = 700;

interface SectionItem {
  value: Section;
  icon: IconName;
}

/**
 * A game's settings in Big Picture: full screen, sections down the left, big controls on the
 * right. There is no Save button: every change is written a moment after it is made, as on a console.
 */
export function GameSettings({ game, onClose }: { game: Game; onClose: () => void }) {
  const { t } = useTranslation();
  const isManual = game.source === "wine" || game.source === "native";
  const isNative = game.source === "native";
  const ref = useAutofocus<HTMLDivElement>();
  const [section, setSection] = useState<Section>(isManual ? "general" : "artwork");
  const { settings, details, setDetails, runners, tools, dirty, valid, change, changeConfig, save } = useGameSettings(
    game.id,
    isManual,
    onClose,
  );
  const pending = dirty;
  const pendingValid = valid;
  const writePending = save;

  // The tile that opened the settings gets the focus back.
  useEffect(() => {
    const opener = document.activeElement;
    return () => {
      if (opener instanceof HTMLElement) opener.focus({ preventScroll: true });
    };
  }, []);

  useEffect(() => {
    if (!pending || !pendingValid) return;
    const timer = window.setTimeout(writePending, AUTOSAVE_MS);
    return () => window.clearTimeout(timer);
  }, [pending, pendingValid, writePending, settings, details]);

  // Leaving writes what is pending first, so nothing typed a moment ago is lost.
  const leave = () => {
    if (pending && pendingValid) writePending();
    onClose();
  };

  useInputAction((action) => {
    if (action === "back" || action === "options") {
      leave();
      return true;
    }
  });

  const sections: SectionItem[] = isManual
    ? [
        { value: "general", icon: "info" },
        { value: "details", icon: "info" },
        ...(isNative ? [] : [{ value: "wine" as const, icon: "settings" as const }]),
        { value: "performance", icon: "settings" },
        { value: "launch", icon: "play" },
        { value: "controller", icon: "gamepad" },
        { value: "artwork", icon: "image" },
        { value: "logs", icon: "terminal" },
      ]
    : [{ value: "artwork", icon: "image" }];
  const label = (value: Section) =>
    t(`config.tab${value === "wine" ? "Wine" : value[0].toUpperCase() + value.slice(1)}`);

  return (
    <div
      ref={ref}
      className={styles.root}
      style={{ "--hue": game.hue } as CSSProperties}
      role="dialog"
      aria-modal="true"
      aria-label={game.title}
      data-nav-trap
    >
      <div className={styles.art} aria-hidden="true">
        <GameArt game={game} kind="hero" />
      </div>

      <header className={styles.head}>
        <button type="button" data-focusable className={styles.back} aria-label={t("common.back")} onClick={leave} {...hoverFocus}>
          <Icon name="arrowLeft" size={24} />
        </button>
        <GameLogo game={game} className={styles.logo} />
        <p className={styles.state} aria-live="polite">
          {pending ? t("config.saving") : t("config.saved")}
        </p>
      </header>

      <div className={styles.tabs}>
        <TabBar
          large
          ariaLabel={t("config.sections")}
          items={sections.map((item) => ({ ...item, label: label(item.value) }))}
          value={section}
          onChange={setSection}
        />
      </div>

      <main className={styles.content}>
        {isManual && !settings ? <Spinner /> : null}
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
      </main>
    </div>
  );
}
