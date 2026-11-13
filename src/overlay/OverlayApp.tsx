import { useEffect, useState, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { GameLogo } from "../components/GameLogo/GameLogo";
import { GameArt } from "../components/GameArt/GameArt";
import { Icon, type IconName } from "../components/Icon/Icon";
import { LogViewer } from "../components/LogViewer/LogViewer";
import { hoverFocus } from "../input/hoverFocus";
import { emitAction } from "../input/bus";
import type { InputAction } from "../input/actions";
import { useAutofocus } from "../input/useAutofocus";
import { useInputAction } from "../input/useInputAction";
import { useKeyboardInput } from "../input/useKeyboardInput";
import { closeOverlay, listGames, onGameRunning, runningGames, stopGameBackend } from "../lib/backend";
import { onControllerAction } from "../lib/controllers";
import { cx } from "../lib/cx";
import { formatLastPlayed, formatPlaytime } from "../lib/format";
import { useLibraryStore } from "../state/library";
import { reportError } from "../state/toasts";
import { ControllerTab } from "../shells/classic/config/ControllerTab";
import type { Game } from "../types/game";
import styles from "./OverlayApp.module.scss";

type Pane = "details" | "controller" | "logs";

interface Entry {
  key: string;
  icon: IconName;
  pane?: Pane;
}

const ENTRIES: Entry[] = [
  { key: "resume", icon: "play" },
  { key: "details", icon: "info", pane: "details" },
  { key: "controller", icon: "gamepad", pane: "controller" },
  { key: "logs", icon: "terminal", pane: "logs" },
  { key: "stop", icon: "stop" },
  { key: "lutra", icon: "monitor" },
];

/** The game being played: the first running one (installers are not games). */
function useRunningGame(): Game | null {
  const games = useLibraryStore((state) => state.games);
  const setGames = useLibraryStore((state) => state.setGames);
  const [runningId, setRunningId] = useState<string | null>(null);

  useEffect(() => {
    listGames()
      .then(setGames)
      .catch((error) => reportError("overlay", error));
    runningGames()
      .then((ids) => setRunningId(ids.find((id) => !id.startsWith("install:")) ?? null))
      .catch(() => setRunningId(null));
    const stop = onGameRunning(({ id, running }) => {
      if (!running && id === runningId) void closeOverlay(false);
      if (running && !id.startsWith("install:")) setRunningId(id);
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [setGames, runningId]);

  return games.find((game) => game.id === runningId) ?? null;
}

/** The in-game overlay: dims the game, lists what can be done without leaving it. */
export function OverlayApp() {
  const { t, i18n } = useTranslation();
  const game = useRunningGame();
  const ref = useAutofocus<HTMLDivElement>();
  const [pane, setPane] = useState<Pane | null>(null);
  const clock = new Intl.DateTimeFormat(i18n.language, { hour: "2-digit", minute: "2-digit" }).format(new Date());

  useKeyboardInput();
  useEffect(() => {
    document.documentElement.dataset.overlay = "";
  }, []);
  // The controller driver sends the overlay the same actions as the keyboard.
  useEffect(() => {
    const stop = onControllerAction((action) => emitAction(action as InputAction));
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  // Back leaves a pane first, then the overlay: the game resumes.
  useInputAction((action) => {
    if (action === "back" || action === "menu") {
      if (pane) setPane(null);
      else void closeOverlay(false);
      return true;
    }
  });

  const run = (entry: Entry) => {
    if (entry.pane) return setPane(entry.pane);
    if (entry.key === "resume") return void closeOverlay(false);
    if (entry.key === "stop" && game) {
      stopGameBackend(game.id).catch((error) => reportError(t("errors.stop"), error));
      return void closeOverlay(false);
    }
    if (entry.key === "lutra") return void closeOverlay(true);
  };

  return (
    <div ref={ref} className={styles.root} style={{ "--hue": game?.hue ?? 220 } as CSSProperties} data-nav-trap>
      {game && (
        <div className={styles.art} aria-hidden="true">
          <GameArt game={game} kind="hero" />
        </div>
      )}
      <time className={styles.clock}>{clock}</time>

      <nav className={styles.menu} aria-label={t("overlay.title")}>
        {game && <GameLogo game={game} className={styles.logo} />}
        <ul>
          {ENTRIES.map((entry) => (
            <li key={entry.key}>
              <button
                type="button"
                data-focusable
                data-autofocus={entry.key === "resume" ? "" : undefined}
                className={cx(styles.item, entry.pane === pane && styles.current)}
                onClick={() => run(entry)}
                {...hoverFocus}
              >
                <Icon name={entry.icon} size={24} />
                {t(`overlay.${entry.key}`)}
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <section className={styles.pane}>
        {pane === "details" && game && <Details game={game} />}
        {pane === "controller" && game && <ControllerTab gameId={game.id} />}
        {pane === "logs" && game && <LogViewer target={game.id} live />}
      </section>
    </div>
  );
}

function Details({ game }: { game: Game }) {
  const { t, i18n } = useTranslation();
  const cards = [
    { label: t("game.playtime"), value: formatPlaytime(game.playtimeMinutes) },
    { label: t("game.lastPlayed"), value: formatLastPlayed(game.lastPlayed, i18n.language, t("game.never")) },
    { label: t("game.source"), value: t(`sources.${game.source}`) },
    game.developer ? { label: t("game.developer"), value: game.developer } : null,
    game.year ? { label: t("game.released"), value: String(game.year) } : null,
  ].filter((card) => card !== null);

  return (
    <>
      {game.description && <p className={styles.description}>{game.description}</p>}
      <div className={styles.cards}>
        {cards.map((card) => (
          <div key={card.label} className={styles.card}>
            <span>{card.label}</span>
            <strong>{card.value}</strong>
          </div>
        ))}
      </div>
    </>
  );
}
