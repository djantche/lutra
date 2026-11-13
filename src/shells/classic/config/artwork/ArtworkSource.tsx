import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../../components/Button/Button";
import { Icon } from "../../../../components/Icon/Icon";
import { Spinner } from "../../../../components/Spinner/Spinner";
import { TextField } from "../../../../components/TextField/TextField";
import { pickArtwork, searchArtwork, type SearchHit } from "../../../../lib/backend";
import { cx } from "../../../../lib/cx";
import { reportError } from "../../../../state/toasts";
import type { Game } from "../../../../types/game";
import styles from "./Artwork.module.scss";

export type MatchState =
  | { status: "loading" }
  | { status: "found"; hit: SearchHit }
  | { status: "none" | "no_key" | "invalid_key" | "failed" };

const SEARCH_DELAY_MS = 400;

interface ArtworkSourceProps {
  game: Game;
  match: MatchState;
  onPicked: (hit: SearchHit) => void;
}

function HitLabel({ hit }: { hit: SearchHit }) {
  return (
    <>
      <span className={styles.hitName}>{hit.name}</span>
      <span className={styles.hitMeta}>
        {hit.year ?? ""}
        {hit.verified && <Icon name="check" size={14} />}
      </span>
    </>
  );
}

/** Which SteamGridDB game the images come from, with a search to pick another one. */
export function ArtworkSource({ game, match, onPicked }: ArtworkSourceProps) {
  const { t } = useTranslation();
  const [searching, setSearching] = useState(false);
  const [query, setQuery] = useState(game.title);
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!searching || query.trim().length < 2) {
      setHits([]);
      return;
    }
    let current = true;
    setLoading(true);
    const timer = window.setTimeout(() => {
      searchArtwork(query)
        .then((found) => current && setHits(found))
        .catch((error) => current && reportError(t("config.artworkFailed"), error))
        .finally(() => current && setLoading(false));
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [query, searching, t]);

  const pick = (sgdbGame: number | null, hit?: SearchHit) => {
    setSearching(false);
    pickArtwork(game.id, sgdbGame)
      .then(() => hit && onPicked(hit))
      .catch((error) => reportError(t("errors.artwork"), error));
  };

  if (match.status === "no_key" || match.status === "invalid_key") {
    return (
      <p className={styles.notice}>
        {match.status === "no_key" ? t("config.artworkNoKey") : t("settings.keyInvalid")}
      </p>
    );
  }

  return (
    <section className={styles.source}>
      <div className={styles.sourceRow}>
        <div className={styles.sourceText}>
          <span className={styles.label}>
            {game.sgdbGameId !== undefined ? t("config.artworkPicked") : t("config.artworkAutomatic")}
          </span>
          <div className={styles.match}>
            {match.status === "loading" && <Spinner />}
            {match.status === "found" && <HitLabel hit={match.hit} />}
            {match.status === "none" && <span>{t("config.artworkNoMatch")}</span>}
            {match.status === "failed" && <span>{t("config.artworkFailed")}</span>}
          </div>
        </div>
        {game.sgdbGameId !== undefined && (
          <Button variant="ghost" icon="refresh" onClick={() => pick(null)}>
            {t("config.artworkUseAutomatic")}
          </Button>
        )}
        <Button variant="secondary" icon="search" onClick={() => setSearching((open) => !open)}>
          {t("config.artworkChange")}
        </Button>
      </div>

      {searching && (
        <div className={styles.search}>
          <TextField
            icon="search"
            clearable
            value={query}
            onChange={setQuery}
            placeholder={t("config.artworkSearch")}
            aria-label={t("config.artworkSearch")}
            spellCheck={false}
            autoFocus
          />
          <div className={styles.hits} aria-busy={loading}>
            {loading && hits.length === 0 && (
              <p className={styles.notice}>
                <Spinner /> {t("config.artworkSearching")}
              </p>
            )}
            {!loading && hits.length === 0 && query.trim().length >= 2 && (
              <p className={styles.notice}>{t("config.artworkNone")}</p>
            )}
            {hits.map((hit) => (
              <button
                key={hit.id}
                type="button"
                data-focusable
                className={cx(styles.hit, match.status === "found" && hit.id === match.hit.id && styles.current)}
                onClick={() => pick(hit.id, hit)}
              >
                <HitLabel hit={hit} />
              </button>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
