import type { CSSProperties } from "react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { SourceLabel } from "../../components/SourceLabel/SourceLabel";
import { GameArt } from "../../components/GameArt/GameArt";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { Icon } from "../../components/Icon/Icon";
import { hoverFocus } from "../../input/hoverFocus";
import { useGameActions } from "../../hooks/useGameActions";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import { formatLastPlayed, formatPlaytime } from "../../lib/format";
import { useLibrary } from "../../state/library";
import type { Game } from "../../types/game";
import styles from "./GameHub.module.scss";

/** Full-screen game page, like a console's game hub. */
export function GameHub({ game, onBack, onSettings }: { game: Game; onBack: () => void; onSettings: () => void }) {
  const { t, i18n } = useTranslation();
  const { toggleFavorite } = useLibrary();
  const { running, label, toggle } = useGameActions(game.id);
  const ref = useAutofocus<HTMLDivElement>();

  // Declared before the autofocus runs, so we remember (and later restore) the tile that opened us.
  useEffect(() => {
    const opener = document.activeElement;
    return () => {
      if (opener instanceof HTMLElement) opener.focus({ preventScroll: true });
    };
  }, []);

  useInputAction((action) => {
    if (action === "back") {
      onBack();
      return true;
    }
    if (action === "options") {
      onSettings();
      return true;
    }
  });

  const favoriteLabel = game.favorite ? t("game.removeFavorite") : t("game.addFavorite");
  const cards = [
    { label: t("game.playtime"), value: formatPlaytime(game.playtimeMinutes) },
    { label: t("game.lastPlayed"), value: formatLastPlayed(game.lastPlayed, i18n.language, t("game.never")) },
    game.developer
      ? { label: t("game.developer"), value: game.developer }
      : { label: t("game.source"), value: t(`sources.${game.source}`) },
    game.year ? { label: t("game.released"), value: String(game.year) } : null,
  ].filter((card) => card !== null);

  return (
    <div
      ref={ref}
      className={styles.hub}
      style={{ "--hue": game.hue } as CSSProperties}
      role="dialog"
      aria-modal="true"
      aria-label={game.title}
      data-nav-trap
    >
      <div className={styles.art} aria-hidden="true">
        <GameArt game={game} kind="hero" />
      </div>

      <button
        type="button"
        data-focusable
        className={styles.back}
        aria-label={t("common.back")}
        onClick={onBack}
        {...hoverFocus}
      >
        <Icon name="arrowLeft" size={24} />
      </button>

      <div className={styles.content}>
        <SourceLabel source={game.source} className={styles.source} />
        <GameLogo game={game} className={styles.logo} />

        <div className={styles.actions}>
          <button
            type="button"
            data-focusable
            data-autofocus
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
            aria-pressed={game.favorite}
            aria-label={favoriteLabel}
            onClick={() => toggleFavorite(game.id)}
            {...hoverFocus}
          >
            <Icon name={game.favorite ? "starFilled" : "star"} size={24} />
          </button>
          <button
            type="button"
            data-focusable
            className={styles.round}
            aria-label={t("config.title", { title: game.title })}
            onClick={onSettings}
            {...hoverFocus}
          >
            <Icon name="settings" size={24} />
          </button>
        </div>

        <div className={styles.cards}>
          {cards.map((card) => (
            <div key={card.label} className={styles.card}>
              <span>{card.label}</span>
              <strong>{card.value}</strong>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
