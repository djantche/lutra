import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SegmentedControl } from "../../../../components/SegmentedControl/SegmentedControl";
import { Spinner } from "../../../../components/Spinner/Spinner";
import { setArtwork, type ArtworkChoices } from "../../../../lib/backend";
import { cx } from "../../../../lib/cx";
import { useLibraryStore } from "../../../../state/library";
import { reportError } from "../../../../state/toasts";
import type { Game } from "../../../../types/game";
import { ArtworkThumb } from "./ArtworkThumb";
import styles from "./Artwork.module.scss";

type Kind = "poster" | "hero";

/** Every cover or banner available for the game; clicking one downloads it right away. */
export function ArtworkGallery({ game, choices }: { game: Game; choices: ArtworkChoices | null }) {
  const { t } = useTranslation();
  const upsertGame = useLibraryStore((state) => state.upsertGame);
  const [kind, setKind] = useState<Kind>("poster");
  const [busyUrl, setBusyUrl] = useState<string | null>(null);

  const candidates = choices ? (kind === "poster" ? choices.posters : choices.heroes) : [];
  const currentUrl = kind === "poster" ? game.posterUrl : game.heroUrl;

  const choose = (url: string) => {
    setBusyUrl(url);
    setArtwork(game.id, kind, url)
      .then(upsertGame)
      .catch((error) => reportError(t("errors.artwork"), error))
      .finally(() => setBusyUrl(null));
  };

  return (
    <section className={styles.gallery}>
      <div className={styles.galleryHeader}>
        <SegmentedControl
          size="sm"
          ariaLabel={t("config.tabArtwork")}
          segments={[
            { value: "poster", label: t("config.covers") },
            { value: "hero", label: t("config.banners") },
          ]}
          value={kind}
          onChange={setKind}
        />
        {choices && <span className={styles.count}>{t("config.imagesCount", { count: candidates.length })}</span>}
      </div>

      {choices === null ? (
        <p className={styles.notice}>
          <Spinner /> {t("config.artworkLoading")}
        </p>
      ) : candidates.length === 0 ? (
        <p className={styles.notice}>{t("config.artworkEmpty")}</p>
      ) : (
        <div className={cx(styles.grid, kind === "hero" && styles.gridWide)}>
          {candidates.map((candidate) => (
            <ArtworkThumb
              key={candidate.url}
              candidate={candidate}
              kind={kind}
              selected={candidate.url === currentUrl}
              busy={candidate.url === busyUrl}
              disabled={busyUrl !== null}
              onChoose={() => choose(candidate.url)}
            />
          ))}
        </div>
      )}
    </section>
  );
}
