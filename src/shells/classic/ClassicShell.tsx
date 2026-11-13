import { useCallback, useEffect, useRef, useState } from "react";
import { useUi } from "../../state/ui";
import { useInputAction } from "../../input/useInputAction";
import { SettingsView } from "../../views/SettingsView";
import { ControllersView } from "../../views/controllers/ControllersView";
import { RemoveGameDialog } from "../../components/RemoveGameDialog/RemoveGameDialog";
import { ContextMenuHost } from "../../components/ContextMenu/ContextMenuHost";
import { ToastHost } from "../../components/Toast/ToastHost";
import { usePageMenu } from "../../hooks/useGameMenu";
import { useLibraryStore } from "../../state/library";
import { AddGameDialog } from "./AddGameDialog";
import { GameConfigDialog } from "./config/GameConfigDialog";
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
        <AddGameDialog onClose={() => setAddGameOpen(false)} onAdded={openGame} />
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
