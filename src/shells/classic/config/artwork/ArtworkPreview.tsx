import { useTranslation } from "react-i18next";
import { GameArt } from "../../../../components/GameArt/GameArt";
import type { Game } from "../../../../types/game";
import styles from "./Artwork.module.scss";

/** The cover and banner currently used, side by side and at their real proportions. */
export function ArtworkPreview({ game }: { game: Game }) {
  const { t } = useTranslation();

  return (
    <div className={styles.preview}>
      <figure className={styles.previewCover}>
        <div className={styles.frame}>
          <GameArt game={game} />
        </div>
        <figcaption>{t("config.cover")}</figcaption>
      </figure>
      <figure className={styles.previewHero}>
        <div className={styles.frame}>
          <GameArt game={game} kind="hero" />
        </div>
        <figcaption>{t("config.banner")}</figcaption>
      </figure>
    </div>
  );
}
