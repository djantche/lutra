import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { EmptyLibrary } from "../../components/EmptyLibrary/EmptyLibrary";
import { GameArt } from "../../components/GameArt/GameArt";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { formatLastPlayed, formatPlaytime } from "../../lib/format";
import { recentGames } from "../../lib/library";
import { useLibrary } from "../../state/library";
import { GameCard, WideCard } from "./GameCard";
import type { Page } from "./navigation";
import { FavoriteButton, PlayButton } from "./PlayButton";
import styles from "./HomePage.module.scss";

interface HomePageProps {
  onOpenGame: (id: string) => void;
  onNavigate: (page: Page) => void;
}

export function HomePage({ onOpenGame, onNavigate }: HomePageProps) {
  const { t, i18n } = useTranslation();
  const { games } = useLibrary();

  const recent = recentGames(games, 5);
  const hero = recent[0] ?? games[0];
  const shelf = recent.slice(1, 5);
  const favorites = games.filter((game) => game.favorite);

  if (!hero) return <EmptyLibrary />;

  return (
    <div className={styles.home} style={{ "--hue": hero.hue } as CSSProperties}>
      <div className={styles.ambient} aria-hidden="true" />

      <section className={styles.hero}>
        <button type="button" className={styles.heroArt} tabIndex={-1} aria-hidden="true" onClick={() => onOpenGame(hero.id)}>
          <GameArt game={hero} kind="hero" />
        </button>
        <div className={styles.heroContent}>
          <span className={styles.eyebrow}>{t("home.continue")}</span>
          <GameLogo game={hero} className={styles.heroLogo} />
          <p className={styles.heroMeta}>
            <span>{t(`sources.${hero.source}`)}</span>
            <span>{formatPlaytime(hero.playtimeMinutes)}</span>
            <span>{formatLastPlayed(hero.lastPlayed, i18n.language, t("game.never"))}</span>
          </p>
          <div className={styles.heroActions}>
            <PlayButton game={hero} size="lg" />
            <Button variant="glass" size="lg" onClick={() => onOpenGame(hero.id)}>
              {t("home.viewGame")}
            </Button>
            <FavoriteButton game={hero} size="lg" />
          </div>
        </div>
      </section>

      {shelf.length > 0 && (
        <section className={styles.section}>
          <header>
            <h2>{t("home.recent")}</h2>
            <Button variant="ghost" onClick={() => onNavigate({ name: "library", filter: "recent" })}>
              {t("home.seeAll")}
            </Button>
          </header>
          <div className={styles.wideGrid}>
            {shelf.map((game) => (
              <WideCard key={game.id} game={game} onOpen={onOpenGame} />
            ))}
          </div>
        </section>
      )}

      {favorites.length > 0 && (
        <section className={styles.section}>
          <header>
            <h2>{t("nav.favorites")}</h2>
            <Button variant="ghost" onClick={() => onNavigate({ name: "library", filter: "favorites" })}>
              {t("home.seeAll")}
            </Button>
          </header>
          <div className={styles.coverGrid}>
            {favorites.map((game) => (
              <GameCard key={game.id} game={game} onOpen={onOpenGame} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
