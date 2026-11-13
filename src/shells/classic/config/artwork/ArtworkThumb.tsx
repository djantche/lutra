import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../../../components/Icon/Icon";
import { Spinner } from "../../../../components/Spinner/Spinner";
import type { Candidate } from "../../../../lib/backend";
import { cx } from "../../../../lib/cx";
import styles from "./Artwork.module.scss";

interface ArtworkThumbProps {
  candidate: Candidate;
  kind: "poster" | "hero";
  selected: boolean;
  busy: boolean;
  disabled: boolean;
  onChoose: () => void;
}

/** One image of the gallery, shown at the proportions it will have once chosen. */
export function ArtworkThumb({ candidate, kind, selected, busy, disabled, onChoose }: ArtworkThumbProps) {
  const { t } = useTranslation();
  const [broken, setBroken] = useState(false);
  // A Steam game without official artwork answers 404: hide the empty tile instead of showing it broken.
  if (broken) return null;

  const official = candidate.style === "official";
  const size = candidate.width && candidate.height ? `${candidate.width} × ${candidate.height}` : null;
  const by = official ? "Steam" : candidate.author ? t("config.by", { author: candidate.author }) : "SteamGridDB";

  return (
    <button
      type="button"
      data-focusable
      className={cx(styles.thumb, kind === "hero" && styles.thumbWide, selected && styles.selected)}
      aria-pressed={selected}
      aria-label={by}
      title={[by, size].filter(Boolean).join(" · ")}
      disabled={disabled && !busy}
      onClick={onChoose}
    >
      <img src={candidate.thumb} alt="" loading="lazy" draggable={false} onError={() => setBroken(true)} />
      {official && <span className={styles.tag}>Steam</span>}
      {selected && (
        <span className={styles.check}>
          <Icon name="check" size={14} />
        </span>
      )}
      {busy && (
        <span className={styles.busy}>
          <Spinner />
        </span>
      )}
      <span className={styles.caption}>{[by, size].filter(Boolean).join(" · ")}</span>
    </button>
  );
}
