import { cx } from "../../lib/cx";
import { useArtworkUrl } from "../../lib/useArtworkUrl";
import type { Game } from "../../types/game";
import styles from "./GameLogo.module.scss";

/**
 * The game's logo (transparent title art from Steam or SteamGridDB), or its name set in type
 * until there is one. It always fills the same box, whatever the logo's shape: the parent sets
 * `--logo-width` and `--logo-height` (and `--logo-align`), and the font-size of the name.
 */
export function GameLogo({ game, className }: { game: Game; className?: string }) {
  const url = useArtworkUrl(game.logo, game.artVersion);
  // Long names are set smaller, so they stay inside the box.
  const length = game.title.length > 30 ? "long" : game.title.length > 18 ? "medium" : undefined;
  return (
    <span className={cx(styles.box, className)}>
      {url ? (
        <img src={url} alt={game.title} draggable={false} className={styles.image} />
      ) : (
        <span className={styles.name} data-length={length}>
          {game.title}
        </span>
      )}
    </span>
  );
}
