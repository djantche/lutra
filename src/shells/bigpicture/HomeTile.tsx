import type { CSSProperties } from "react";
import { GameArt } from "../../components/GameArt/GameArt";
import { useGameMenu } from "../../hooks/useGameMenu";
import { hoverFocus } from "../../input/hoverFocus";
import type { Game } from "../../types/game";
import styles from "./HomeView.module.scss";

interface HomeTileProps {
  game: Game;
  autofocus: boolean;
  onFocusGame: (id: string) => void;
  onOpenGame: (game: Game) => void;
}

/** A square tile of the home shelf. */
export function HomeTile({ game, autofocus, onFocusGame, onOpenGame }: HomeTileProps) {
  const openMenu = useGameMenu(game, () => onOpenGame(game));

  return (
    <button
      type="button"
      data-focusable
      data-autofocus={autofocus ? "" : undefined}
      className={styles.tile}
      style={{ "--hue": game.hue } as CSSProperties}
      aria-label={game.title}
      onFocus={() => onFocusGame(game.id)}
      onClick={() => onOpenGame(game)}
      onContextMenu={openMenu}
      {...hoverFocus}
    >
      <span className={styles.tileArt}>
        <GameArt game={game} />
      </span>
      <span className={styles.tileLabel}>{game.title}</span>
    </button>
  );
}
