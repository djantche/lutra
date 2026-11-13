import { memo, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { GameArt } from "../../components/GameArt/GameArt";
import { Icon } from "../../components/Icon/Icon";
import { SourceLabel } from "../../components/SourceLabel/SourceLabel";
import { useGameActions } from "../../hooks/useGameActions";
import { useGameMenu } from "../../hooks/useGameMenu";
import { cx } from "../../lib/cx";
import { formatLastPlayed, formatPlaytime } from "../../lib/format";
import { useLibraryStore } from "../../state/library";
import type { Game } from "../../types/game";
import styles from "./GameRow.module.scss";

interface GameRowProps {
  game: Game;
  onOpen: (id: string) => void;
  /** Hidden when the page already is that source page. */
  showSource?: boolean;
}

/** One game as a line of the list view: open it, play it, star it, or reach its menu. */
function GameRowView({ game, onOpen, showSource = true }: GameRowProps) {
  const { t, i18n } = useTranslation();
  const openMenu = useGameMenu(game, () => onOpen(game.id));
  const toggleFavorite = useLibraryStore((state) => state.toggleFavorite);
  const { running, label, toggle } = useGameActions(game.id);
  const favoriteLabel = game.favorite ? t("game.removeFavorite") : t("game.addFavorite");

  return (
    <div
      className={cx(styles.row, running && styles.running)}
      style={{ "--hue": game.hue } as CSSProperties}
      onClick={() => onOpen(game.id)}
      onContextMenu={openMenu}
    >
      <span className={styles.cover}>
        <GameArt game={game} />
      </span>

      <button
        type="button"
        data-focusable
        className={styles.title}
        onClick={(event) => {
          event.stopPropagation();
          onOpen(game.id);
        }}
      >
        <strong>{game.title}</strong>
        <small>
          {running ? (
            <span className={styles.live}>{t("game.running")}</span>
          ) : (
            (game.developer ?? (showSource ? null : t(`sources.${game.source}`)))
          )}
        </small>
      </button>

      <span className={styles.cell}>{showSource && <SourceLabel source={game.source} />}</span>
      <span className={cx(styles.cell, styles.number)}>
        {game.playtimeMinutes > 0 ? formatPlaytime(game.playtimeMinutes) : "—"}
      </span>
      <span className={styles.cell}>{formatLastPlayed(game.lastPlayed, i18n.language, t("game.never"))}</span>

      <span className={styles.actions} onClick={(event) => event.stopPropagation()}>
        <button
          type="button"
          className={cx(styles.icon, game.favorite && styles.starred)}
          aria-pressed={game.favorite}
          aria-label={favoriteLabel}
          title={favoriteLabel}
          onClick={() => toggleFavorite(game.id)}
        >
          <Icon name={game.favorite ? "starFilled" : "star"} size={16} />
        </button>
        <button
          type="button"
          className={styles.icon}
          aria-label={t("menu.more")}
          title={t("menu.more")}
          onClick={openMenu}
        >
          <Icon name="more" size={16} />
        </button>
        <Button
          variant={running ? "secondary" : "primary"}
          icon={running ? "stop" : "play"}
          className={styles.play}
          onClick={toggle}
        >
          {label}
        </Button>
      </span>
    </div>
  );
}

export const GameRow = memo(GameRowView);
