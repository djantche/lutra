import type { CSSProperties } from "react";
import { GameArt } from "../../components/GameArt/GameArt";
import { useGameMenu } from "../../hooks/useGameMenu";
import { hoverFocus } from "../../input/hoverFocus";
import type { Game } from "../../types/game";
import styles from "./LibraryView.module.scss";

interface LibraryTileProps {
  game: Game;
  autofocus: boolean;
  onFocusGame: (id: string) => void;
  onOpenGame: (game: Game) => void;
}

/** A poster of the library grid. */
export function LibraryTile({ game, autofocus, onFocusGame, onOpenGame }: LibraryTileProps) {
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
      <GameArt game={game} showTitle />
    </button>
  );
}
