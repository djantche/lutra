import { useEffect, useState } from "react";
import { artworkChoices, artworkMatch, type ArtworkChoices, type SearchHit } from "../../../../lib/backend";
import type { Game } from "../../../../types/game";
import { ArtworkGallery } from "./ArtworkGallery";
import { ArtworkPreview } from "./ArtworkPreview";
import { ArtworkSource, type MatchState } from "./ArtworkSource";
import styles from "./Artwork.module.scss";

/**
 * Shows the game's current images, which SteamGridDB game they come from, and every image
 * available, so the user sees exactly what gets downloaded before choosing it.
 */
export function ArtworkTab({ game }: { game: Game }) {
  const [match, setMatch] = useState<MatchState>({ status: "loading" });
  const [choices, setChoices] = useState<ArtworkChoices | null>(null);

  // Which SteamGridDB game is used: the one picked, or the best guess.
  useEffect(() => {
    let current = true;
    setMatch({ status: "loading" });
    artworkMatch(game.id, game.title)
      .then((hit) => current && setMatch(hit ? { status: "found", hit } : { status: "none" }))
      .catch((error) => {
        if (!current) return;
        setMatch(error === "no_key" || error === "invalid_key" ? { status: error } : { status: "failed" });
      });
    return () => {
      current = false;
    };
  }, [game.id, game.title, game.sgdbGameId]);

  const sgdbGame: number | null = match.status === "found" ? match.hit.id : null;
  const settled = match.status !== "loading";

  useEffect(() => {
    if (!settled) return;
    let current = true;
    setChoices(null);
    artworkChoices(game.id, sgdbGame)
      .then((found) => current && setChoices(found))
      .catch(() => current && setChoices({ posters: [], heroes: [] }));
    return () => {
      current = false;
    };
  }, [game.id, sgdbGame, settled]);

  const onPicked = (hit: SearchHit) => setMatch({ status: "found", hit });

  return (
    <div className={styles.tab}>
      <ArtworkPreview game={game} />
      <ArtworkSource game={game} match={match} onPicked={onPicked} />
      <ArtworkGallery game={game} choices={choices} />
    </div>
  );
}
