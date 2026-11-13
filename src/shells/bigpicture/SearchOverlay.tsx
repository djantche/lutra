import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { VirtualGrid } from "../../components/VirtualGrid/VirtualGrid";
import { VirtualKeyboard, type KeyboardLayout } from "../../components/VirtualKeyboard/VirtualKeyboard";
import { hoverFocus } from "../../input/hoverFocus";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import { cx } from "../../lib/cx";
import { filterGames, filterTitle, type LibraryFilter } from "../../lib/library";
import { SOURCES } from "../../lib/sources";
import type { Game } from "../../types/game";
import { LibraryTile } from "./LibraryTile";
import { tileWidth } from "./LibraryView";
import { StatusCluster } from "./StatusCluster";
import styles from "./SearchOverlay.module.scss";

/** Letters without their accents, in lower case: "Pokémon" is found by "pokemon". */
const plain = (text: string) =>
  text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();

const matches = (game: Game, query: string) =>
  plain([game.title, game.developer ?? "", ...game.genres].join(" ")).includes(plain(query.trim()));

interface SearchOverlayProps {
  games: Game[];
  onClose: () => void;
  onOpenGame: (game: Game) => void;
  onFocusGame: (id: string) => void;
}

/**
 * Search, as on a console: a bar at the top, filters, the matching games, and (when the bar is
 * confirmed) an on-screen keyboard. The keyboard of a computer types in the bar directly.
 */
export function SearchOverlay({ games, onClose, onOpenGame, onFocusGame }: SearchOverlayProps) {
  const { t, i18n } = useTranslation();
  const ref = useAutofocus<HTMLDivElement>();
  const input = useRef<HTMLInputElement>(null);
  const results = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<LibraryFilter>("all");
  const [keyboard, setKeyboard] = useState(false);
  const layout: KeyboardLayout = i18n.language.startsWith("fr") ? "azerty" : "qwerty";

  const found = useMemo(() => games.filter((game) => matches(game, query)), [games, query]);
  const filters: LibraryFilter[] = [
    "all",
    "favorites",
    ...SOURCES.filter((source) => games.some((game) => game.source === source)),
  ];
  const visible = useMemo(
    () => [...filterGames(found, filter)].sort((a, b) => a.title.localeCompare(b.title)),
    [found, filter],
  );

  const showKeyboard = () => setKeyboard(true);
  const hideKeyboard = () => {
    setKeyboard(false);
    input.current?.focus({ preventScroll: true });
  };
  const toResults = () => {
    setKeyboard(false);
    // Once the keyboard is gone, the first result takes the selection.
    window.requestAnimationFrame(() => results.current?.querySelector<HTMLElement>("[data-focusable]")?.focus());
  };

  useInputAction((action) => {
    // Confirming the bar brings the keyboard up.
    if (action === "confirm" && document.activeElement === input.current) {
      showKeyboard();
      return true;
    }
    if (action === "back") {
      if (keyboard) hideKeyboard();
      else onClose();
      return true;
    }
  });

  return (
    <div ref={ref} className={styles.root} role="dialog" aria-modal="true" aria-label={t("bigPicture.searchOpen")} data-nav-trap>
      <header className={styles.header}>
        <label className={styles.bar}>
          <Icon name="search" size={30} />
          <input
            ref={input}
            data-focusable
            data-autofocus
            className={styles.input}
            value={query}
            placeholder={t("bigPicture.searchPlaceholder")}
            spellCheck={false}
            autoComplete="off"
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") toResults();
            }}
            {...hoverFocus}
          />
        </label>
        <button
          type="button"
          data-focusable
          className={styles.keys}
          aria-label={t("keyboard.label")}
          aria-pressed={keyboard}
          onClick={() => (keyboard ? hideKeyboard() : showKeyboard())}
          {...hoverFocus}
        >
          <Icon name="keyboard" size={30} />
        </button>
        <div className={styles.status}>
          <StatusCluster />
        </div>
      </header>

      <nav className={styles.filters} aria-label={t("bigPicture.searchFilters")}>
        {filters.map((item) => (
          <button
            key={item}
            type="button"
            data-focusable
            className={cx(styles.filter, item === filter && styles.current)}
            aria-pressed={item === filter}
            onClick={() => setFilter(item)}
            {...hoverFocus}
          >
            {filterTitle(item, t)}
            <span className={styles.count}>{filterGames(found, item).length}</span>
          </button>
        ))}
      </nav>

      <div ref={results} className={styles.results}>
        {visible.length > 0 ? (
          <VirtualGrid
            items={visible}
            getKey={(game) => game.id}
            minColumnWidth={tileWidth()}
            columnGap={28}
            rowGap={36}
            renderItem={(game) => (
              <LibraryTile game={game} autofocus={false} onFocusGame={onFocusGame} onOpenGame={onOpenGame} />
            )}
          />
        ) : (
          <p className={styles.empty}>{t("bigPicture.searchEmpty")}</p>
        )}
      </div>

      {keyboard && (
        <VirtualKeyboard
          layout={layout}
          onInsert={(text) => setQuery((current) => current + text)}
          onBackspace={() => setQuery((current) => current.slice(0, -1))}
          onClear={() => setQuery("")}
          onSubmit={toResults}
          onHide={hideKeyboard}
        />
      )}
    </div>
  );
}
