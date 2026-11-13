import { useTranslation } from "react-i18next";
import type { GameHit } from "../../lib/backend";
import { cx } from "../../lib/cx";
import { Spinner } from "../Spinner/Spinner";
import styles from "./GameMatches.module.scss";

interface GameMatchesProps {
  hits: GameHit[];
  loading: boolean;
  failed: boolean;
  /** GameDB id of the match in use. */
  selected: number | null;
  onSelect: (hit: GameHit) => void;
}

/** GameDB matches for what was typed. Picking one fills in the details; none is also a valid answer. */
export function GameMatches({ hits, loading, failed, selected, onSelect }: GameMatchesProps) {
  const { t } = useTranslation();

  if (loading && hits.length === 0) {
    return (
      <p className={styles.status}>
        <Spinner /> {t("details.searching")}
      </p>
    );
  }
  if (failed) return <p className={styles.status}>{t("details.lookupFailed")}</p>;
  if (hits.length === 0) return <p className={styles.status}>{t("details.noMatch")}</p>;

  return (
    <ul className={styles.list} role="listbox" aria-label={t("details.matches")}>
      {hits.map((hit) => (
        <li key={hit.id} role="option" aria-selected={hit.id === selected}>
          <button
            type="button"
            data-focusable
            className={cx(styles.hit, hit.id === selected && styles.selected)}
            onClick={() => onSelect(hit)}
          >
            <span className={styles.cover}>{hit.cover && <img src={hit.cover} alt="" loading="lazy" />}</span>
            <span className={styles.text}>
              <strong>{hit.name}</strong>
              <small>{[hit.year, hit.developer].filter(Boolean).join(" · ")}</small>
            </span>
            {hit.pc && <span className={styles.badge}>PC</span>}
          </button>
        </li>
      ))}
    </ul>
  );
}
