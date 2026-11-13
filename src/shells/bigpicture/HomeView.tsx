import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { EmptyLibrary } from "../../components/EmptyLibrary/EmptyLibrary";
import { SourceLabel } from "../../components/SourceLabel/SourceLabel";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { Icon } from "../../components/Icon/Icon";
import { hoverFocus } from "../../input/hoverFocus";
import { useAutofocus } from "../../input/useAutofocus";
import { formatLastPlayed, formatPlaytime } from "../../lib/format";
import { recentGames } from "../../lib/library";
import { useLibrary } from "../../state/library";
import type { Game } from "../../types/game";
import { useGameActions } from "../../hooks/useGameActions";
import { HomeTile } from "./HomeTile";
import styles from "./HomeView.module.scss";

interface HomeViewProps {
  focused: Game | null;
  onFocusGame: (id: string) => void;
  onOpenGame: (game: Game) => void;
  onShowLibrary: () => void;
}

export function HomeView({ focused, onFocusGame, onOpenGame, onShowLibrary }: HomeViewProps) {
  const { t, i18n } = useTranslation();
  const { games, toggleFavorite } = useLibrary();
  const { running, label, toggle } = useGameActions(focused?.id ?? "");
  const ref = useAutofocus<HTMLDivElement>();

  const recent = recentGames(games, 10);
  const current = focused ?? recent[0] ?? null;

  useEffect(() => {
    if (!focused && recent[0]) onFocusGame(recent[0].id);
  }, []);

  if (games.length === 0) {
    return (
      <div ref={ref} className={styles.home}>
        <EmptyLibrary large />
      </div>
    );
  }

  return (
    <div ref={ref} className={styles.home}>
      <div
        className={styles.shelf}
        onWheel={(event) => {
          event.currentTarget.scrollLeft += event.deltaY;
        }}
      >
        {recent.map((game, index) => (
          <HomeTile
            key={game.id}
            game={game}
            autofocus={index === 0}
            onFocusGame={onFocusGame}
            onOpenGame={onOpenGame}
          />
        ))}

        <button
          type="button"
          data-focusable
          className={styles.tile}
          aria-label={t("nav.library")}
          onClick={onShowLibrary}
          {...hoverFocus}
        >
          <span className={styles.libraryTile}>
            <Icon name="grid" size={34} />
          </span>
          <span className={styles.tileLabel}>{t("nav.library")}</span>
        </button>
      </div>

      {current && (
        <section key={current.id} className={styles.info}>
          <GameLogo game={current} className={styles.logo} />

          <div className={styles.meta}>
            <SourceLabel source={current.source} className={styles.chip} />
            <span>{formatPlaytime(current.playtimeMinutes)}</span>
            <span>{formatLastPlayed(current.lastPlayed, i18n.language, t("game.never"))}</span>
          </div>

          <div className={styles.actions}>
            <button
              type="button"
              data-focusable
              className={styles.play}
              onClick={toggle}
              {...hoverFocus}
            >
              <Icon name={running ? "stop" : "play"} size={24} />
              {label}
            </button>
            <button
              type="button"
              data-focusable
              className={styles.round}
              aria-label={t("game.details")}
              onClick={() => onOpenGame(current)}
              {...hoverFocus}
            >
              <Icon name="info" size={24} />
              <span className={styles.roundLabel}>{t("game.details")}</span>
            </button>
            <button
              type="button"
              data-focusable
              className={styles.round}
              aria-pressed={current.favorite}
              aria-label={current.favorite ? t("game.removeFavorite") : t("game.addFavorite")}
              onClick={() => toggleFavorite(current.id)}
              {...hoverFocus}
            >
              <Icon name={current.favorite ? "starFilled" : "star"} size={24} />
              <span className={styles.roundLabel}>
                {current.favorite ? t("game.removeFavorite") : t("game.addFavorite")}
              </span>
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
