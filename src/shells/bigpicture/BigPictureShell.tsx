import { useEffect, useState } from "react";
import { ContextMenuHost } from "../../components/ContextMenu/ContextMenuHost";
import { ToastHost } from "../../components/Toast/ToastHost";
import { GameArt } from "../../components/GameArt/GameArt";
import { useIdleCursor } from "../../input/useIdleCursor";
import { useInputAction } from "../../input/useInputAction";
import { cx } from "../../lib/cx";
import { useLibrary } from "../../state/library";
import { useUi } from "../../state/ui";
import type { Game } from "../../types/game";
import { SettingsView } from "../../views/SettingsView";
import { ControllersView } from "../../views/controllers/ControllersView";
import { BigPictureMenu } from "./BigPictureMenu";
import { GameHub } from "./GameHub";
import { GameSettings } from "./GameSettings";
import { HomeView } from "./HomeView";
import { LaunchScreen } from "./LaunchScreen";
import { LibraryView } from "./LibraryView";
import { Splash } from "./Splash";
import { TopBar, TABS, type Tab } from "./TopBar";
import styles from "./BigPictureShell.module.scss";

/** Keeps the previous artwork underneath so backgrounds cross-fade instead of flashing. */
function Backdrop({ game, dim }: { game: Game | null; dim: boolean }) {
  const [layers, setLayers] = useState<Game[]>(game ? [game] : []);

  useEffect(() => {
    if (!game) return;
    setLayers((current) => (current[current.length - 1]?.id === game.id ? current : [...current.slice(-1), game]));
  }, [game]);

  return (
    <div className={cx(styles.backdrop, dim && styles.dim)} aria-hidden="true">
      {layers.map((layer) => (
        <div key={layer.id} className={styles.backdropLayer}>
          <GameArt game={layer} kind="hero" />
        </div>
      ))}
    </div>
  );
}

export function BigPictureShell() {
  const { games, launching } = useLibrary();
  const [tab, setTab] = useState<Tab>("home");
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [hubId, setHubId] = useState<string | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [settingsId, setSettingsId] = useState<string | null>(null);
  const { setMode } = useUi();
  const idle = useIdleCursor();

  const focused = games.find((game) => game.id === focusedId) ?? null;
  const hub = games.find((game) => game.id === hubId) ?? null;
  const settingsGame = games.find((game) => game.id === settingsId) ?? null;
  const openGame = (game: Game) => setHubId(game.id);

  useInputAction(
    (action) => {
      if (action === "menu") {
        setMenuOpen(true);
        return true;
      }
      // Options / Select on a game: its settings (the home and library pages track the selected tile).
      if (action === "options" && focused) {
        setSettingsId(focused.id);
        return true;
      }
      if (action === "tabNext" || action === "tabPrev") {
        const step = action === "tabNext" ? 1 : -1;
        const from = Math.max(0, TABS.indexOf(tab));
        setTab(TABS[(from + step + TABS.length) % TABS.length]);
        return true;
      }
      if (action === "back" && tab !== "home") {
        setTab("home");
        return true;
      }
    },
    hub === null && launching === null && !menuOpen && settingsGame === null,
  );

  return (
    <div className={styles.shell} data-idle={idle || undefined}>
      <Backdrop game={focused} dim={tab !== "home"} />

      <div className={styles.chrome} inert={hub !== null || menuOpen || settingsGame !== null}>
        <TopBar tab={tab} onTab={setTab} onMenu={() => setMenuOpen(true)} />
        <main className={styles.content}>
          <div key={tab} className={styles.view}>
            {tab === "home" && (
              <HomeView
                focused={focused}
                onFocusGame={setFocusedId}
                onOpenGame={openGame}
                onShowLibrary={() => setTab("library")}
              />
            )}
            {tab === "library" && <LibraryView onFocusGame={setFocusedId} onOpenGame={openGame} />}
            {tab === "controllers" && (
              <div className={styles.settings}>
                <ControllersView />
              </div>
            )}
            {tab === "settings" && (
              <div className={styles.settings}>
                <SettingsView />
              </div>
            )}
          </div>
        </main>
      </div>

      {menuOpen && (
        <BigPictureMenu
          tab={tab}
          onTab={setTab}
          onClose={() => setMenuOpen(false)}
          onExit={() => setMode("classic")}
        />
      )}
      {hub && <GameHub game={hub} onBack={() => setHubId(null)} onSettings={() => setSettingsId(hub.id)} />}
      {settingsGame && <GameSettings game={settingsGame} onClose={() => setSettingsId(null)} />}
      {launching && <LaunchScreen game={launching} />}
      <div className={styles.overlayRoot} data-overlay-root />
      <ContextMenuHost />
      <ToastHost />
      <Splash />
    </div>
  );
}
