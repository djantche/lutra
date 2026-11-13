import { memo, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { GameArt } from "../../components/GameArt/GameArt";
import { SourceLabel } from "../../components/SourceLabel/SourceLabel";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { Icon } from "../../components/Icon/Icon";
import { formatLastPlayed } from "../../lib/format";
import { useGameActions } from "../../hooks/useGameActions";
import { useGameMenu } from "../../hooks/useGameMenu";
import { useLibraryStore } from "../../state/library";
import type { Game } from "../../types/game";
import styles from "./GameCard.module.scss";

interface CardProps {
  game: Game;
  onOpen: (id: string) => void;
  /** Hidden when the page already is that source page. */
  showSource?: boolean;
}

function QuickPlay({ game }: { game: Game }) {
  const { t } = useTranslation();
  const { running, toggle } = useGameActions(game.id);
  const label = running ? t("game.stopTitle", { title: game.title }) : t("game.playTitle", { title: game.title });
  return (
    <button type="button" tabIndex={-1} className={styles.quickPlay} aria-label={label} title={label} onClick={toggle}>
      <Icon name={running ? "stop" : "play"} size={18} />
    </button>
  );
}

/** Poster card: box art, title and source. */
function GameCardView({ game, onOpen, showSource = true }: CardProps) {
  const { t } = useTranslation();
  const openMenu = useGameMenu(game, () => onOpen(game.id));
  const running = useLibraryStore((state) => state.runningIds.includes(game.id));

  return (
    <div className={styles.card} style={{ "--hue": game.hue } as CSSProperties} onContextMenu={openMenu}>
      <button type="button" data-focusable className={styles.open} onClick={() => onOpen(game.id)}>
        <span className={styles.cover}>
          <GameArt game={game} showTitle />
          {game.favorite && (
            <span className={styles.favorite}>
              <Icon name="starFilled" size={12} />
            </span>
          )}
          {running && <span className={styles.running}>{t("game.running")}</span>}
        </span>
        <span className={styles.caption}>
          <strong>{game.title}</strong>
          {showSource && <SourceLabel source={game.source} className={styles.source} />}
        </span>
      </button>
      <QuickPlay game={game} />
    </div>
  );
}

/** Landscape card used in the "recently played" shelf. */
function WideCardView({ game, onOpen }: CardProps) {
  const { t, i18n } = useTranslation();
  const openMenu = useGameMenu(game, () => onOpen(game.id));

  return (
    <div className={styles.wide} style={{ "--hue": game.hue } as CSSProperties} onContextMenu={openMenu}>
      <button type="button" data-focusable className={styles.wideOpen} onClick={() => onOpen(game.id)}>
        <GameArt game={game} kind="hero" className={styles.wideArt} />
        <span className={styles.wideInfo}>
          <GameLogo game={game} className={styles.wideLogo} />
          <small>{formatLastPlayed(game.lastPlayed, i18n.language, t("game.never"))}</small>
        </span>
      </button>
      <QuickPlay game={game} />
    </div>
  );
}

// Cards only redraw when their own game changes, e.g. when its artwork arrives.
export const GameCard = memo(GameCardView);
export const WideCard = memo(WideCardView);
