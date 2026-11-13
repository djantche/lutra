import { useCallback, useEffect, useRef, useState } from "react";
import { useUi } from "../../state/ui";
import { useInputAction } from "../../input/useInputAction";
import { SettingsView } from "../../views/SettingsView";
import { ControllersView } from "../../views/controllers/ControllersView";
import { EmulationView } from "../../views/emulation/EmulationView";
import type { Game } from "../../types/game";
import { RemoveGameDialog } from "../../components/RemoveGameDialog/RemoveGameDialog";
import { VirtualGrid } from "../../components/VirtualGrid/VirtualGrid";
import { ContextMenuHost } from "../../components/ContextMenu/ContextMenuHost";
import { ToastHost } from "../../components/Toast/ToastHost";
import { usePageMenu } from "../../hooks/useGameMenu";
import { useLibraryStore } from "../../state/library";
import { AddGameDialog } from "./AddGameDialog";
import { GameConfigDialog } from "./config/GameConfigDialog";
import { GameCard } from "./GameCard";
import { GamePage } from "./GamePage";
import { HomePage } from "./HomePage";
import { LaunchToast } from "./LaunchToast";
import { LibraryPage } from "./LibraryPage";
import { pageKey, type Page } from "./navigation";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import styles from "./ClassicShell.module.scss";

export function ClassicShell() {
  const {
    addGameOpen,
    setAddGameOpen,
    addRomSystem,
    openAddRom,
    configureGameId,
    setConfigureGameId,
    removeGameId,
    setRemoveGameId,
  } = useUi();
  const removing = useLibraryStore((state) => state.games.find((game) => game.id === removeGameId));
  const openPageMenu = usePageMenu();
  const [history, setHistory] = useState<Page[]>([{ name: "home" }]);
  const [query, setQuery] = useState("");
  const [scrolled, setScrolled] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const page = history[history.length - 1];
  const searching = query.trim() !== "";
  const canGoBack = history.length > 1;

  const navigate = useCallback((next: Page) => {
    setQuery("");
    setHistory((current) =>
      pageKey(current[current.length - 1]) === pageKey(next) ? current : [...current, next],
    );
  }, []);

  const back = useCallback(() => {
    setQuery("");
    setHistory((current) => (current.length > 1 ? current.slice(0, -1) : current));
  }, []);

  const openGame = useCallback((id: string) => navigate({ name: "game", id }), [navigate]);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [page, searching]);

  // The "back" side button of the mouse.
  useEffect(() => {
    const onMouseUp = (event: MouseEvent) => {
      if (event.button === 3) back();
    };
    window.addEventListener("mouseup", onMouseUp);
    return () => window.removeEventListener("mouseup", onMouseUp);
  }, [back]);

  useInputAction((action) => {
    if (action !== "back") return;
    if (searching) {
      setQuery("");
      return true;
    }
    if (canGoBack) {
      back();
      return true;
    }
  });

  let content;
  if (searching) {
    content = <LibraryPage filter="all" query={query} onOpenGame={openGame} />;
  } else if (page.name === "home") {
    content = <HomePage onOpenGame={openGame} onNavigate={navigate} />;
  } else if (page.name === "library") {
    content = <LibraryPage filter={page.filter} onOpenGame={openGame} />;
  } else if (page.name === "game") {
    content = <GamePage id={page.id} onOpenGame={openGame} onRemoved={back} />;
  } else if (page.name === "emulation") {
    content = (
      <div className={styles.settings}>
        <EmulationView
          system={page.system ?? null}
          onSystem={(system) => (system ? navigate({ name: "emulation", system }) : back())}
          renderGames={(games: Game[]) => (
            <VirtualGrid
              items={games}
              getKey={(game) => game.id}
              minColumnWidth={170}
              columnGap={20}
              rowGap={32}
              renderItem={(game) => <GameCard game={game} onOpen={openGame} showSource={false} />}
            />
          )}
          onAddRom={openAddRom}
          onOpenSettings={() => navigate({ name: "settings", section: "emulation" })}
        />
      </div>
    );
  } else if (page.name === "controllers") {
    content = (
      <div className={styles.settings}>
        <ControllersView />
      </div>
    );
  } else {
    content = (
      <div className={styles.settings}>
        <SettingsView section={page.name === "settings" ? page.section : undefined} />
      </div>
    );
  }

  return (
    <div className={styles.shell}>
      <Sidebar page={page} onNavigate={navigate} />
      <div className={styles.main}>
        <TitleBar
          scrolled={scrolled}
          canGoBack={canGoBack || searching}
          onBack={back}
          query={query}
          onQuery={setQuery}
        />
        <div
          ref={scrollRef}
          className={styles.scroll}
          onContextMenu={openPageMenu}
          onScroll={(event) => setScrolled(event.currentTarget.scrollTop > 8)}
        >
          <div key={searching ? "search" : pageKey(page)} className={styles.page}>
            {content}
          </div>
        </div>
      </div>
      {addGameOpen && (
        <AddGameDialog romSystem={addRomSystem} onClose={() => setAddGameOpen(false)} onAdded={openGame} />
      )}
      {configureGameId && <GameConfigDialog gameId={configureGameId} onClose={() => setConfigureGameId(null)} />}
      {removing && <RemoveGameDialog game={removing} onClose={() => setRemoveGameId(null)} />}
      <div className={styles.overlayRoot} data-overlay-root />
      <ContextMenuHost />
      <ToastHost />
      <LaunchToast />
    </div>
  );
}
