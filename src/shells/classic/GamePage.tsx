import { useEffect, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { GameArt } from "../../components/GameArt/GameArt";
import { SourceLabel } from "../../components/SourceLabel/SourceLabel";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { formatLastPlayed, formatPlaytime } from "../../lib/format";
import { useLibrary } from "../../state/library";
import { useUi } from "../../state/ui";
import { GameCard } from "./GameCard";
import { FavoriteButton, PlayButton } from "./PlayButton";
import styles from "./GamePage.module.scss";

interface GamePageProps {
  id: string;
  onOpenGame: (id: string) => void;
  onRemoved: () => void;
}

export function GamePage({ id, onOpenGame, onRemoved }: GamePageProps) {
  const { t, i18n } = useTranslation();
  const { games } = useLibrary();
  const { setConfigureGameId, setRemoveGameId } = useUi();
  const game = games.find((candidate) => candidate.id === id);
  // Removed (from this page or its menu): there is nothing left to show.
  useEffect(() => {
    if (!game) onRemoved();
  }, [game, onRemoved]);
  if (!game) return null;

  const related = games.filter((other) => other.source === game.source && other.id !== game.id).slice(0, 6);
  const sourceLabel = t(`sources.${game.source}`);

  const stats = [
    { label: t("game.playtime"), value: formatPlaytime(game.playtimeMinutes) },
    { label: t("game.lastPlayed"), value: formatLastPlayed(game.lastPlayed, i18n.language, t("game.never")) },
    game.developer ? { label: t("game.developer"), value: game.developer } : { label: t("game.source"), value: sourceLabel },
    game.year ? { label: t("game.released"), value: String(game.year) } : null,
  ].filter((stat) => stat !== null);

  return (
    <div className={styles.page} style={{ "--hue": game.hue } as CSSProperties}>
      <div className={styles.ambient} aria-hidden="true" />

      <section className={styles.banner}>
        <GameArt game={game} kind="hero" className={styles.bannerArt} />
        <div className={styles.bannerContent}>
          <SourceLabel source={game.source} className={styles.source} />
          <GameLogo game={game} className={styles.logo} />
          <div className={styles.actions}>
            <PlayButton game={game} size="lg" />
            <FavoriteButton game={game} size="lg" />
            <Button
              variant="glass"
              size="lg"
              icon="settings"
              aria-label={t("menu.configure")}
              title={t("menu.configure")}
              onClick={() => setConfigureGameId(game.id)}
            />
            {(game.source === "wine" || game.source === "native") && (
              <Button
                variant="glass"
                size="lg"
                icon="trash"
                aria-label={t("game.remove")}
                title={t("game.remove")}
                onClick={() => setRemoveGameId(game.id)}
              />
            )}
          </div>
        </div>
      </section>

      <div className={styles.stats}>
        {stats.map((stat) => (
          <div key={stat.label} className={styles.stat}>
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
          </div>
        ))}
      </div>

      {(game.description || game.genres.length > 0) && (
        <section className={styles.about}>
          <h2>{t("game.about")}</h2>
          {game.description && <p>{game.description}</p>}
          {game.genres.length > 0 && (
            <ul>
              {game.genres.map((genre) => (
                <li key={genre}>{genre}</li>
              ))}
            </ul>
          )}
        </section>
      )}

      {related.length > 0 && (
        <section className={styles.related}>
          <h2>{t("game.moreFrom", { source: sourceLabel })}</h2>
          <div className={styles.relatedGrid}>
            {related.map((other) => (
              <GameCard key={other.id} game={other} onOpen={onOpenGame} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
