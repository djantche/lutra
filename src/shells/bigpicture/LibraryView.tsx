import { useState } from "react";
import { useTranslation } from "react-i18next";
import { hoverFocus } from "../../input/hoverFocus";
import { useAutofocus } from "../../input/useAutofocus";
import { VirtualGrid } from "../../components/VirtualGrid/VirtualGrid";
import { filterGames, filterTitle, librarySystems, sortGames, systemFilter, type LibraryFilter } from "../../lib/library";
import { SOURCES } from "../../lib/sources";
import { useLibrary } from "../../state/library";
import type { Game } from "../../types/game";
import { LibraryTile } from "./LibraryTile";
import styles from "./LibraryView.module.scss";

/** A poster's narrowest width in Big Picture: `clamp(160px, 13vw, 220px)`. */
export const tileWidth = () => Math.min(220, Math.max(160, window.innerWidth * 0.13));

interface LibraryViewProps {
  onFocusGame: (id: string) => void;
  onOpenGame: (game: Game) => void;
}

export function LibraryView({ onFocusGame, onOpenGame }: LibraryViewProps) {
  const { t } = useTranslation();
  const { games } = useLibrary();
  const ref = useAutofocus<HTMLDivElement>();
  const [filter, setFilter] = useState<LibraryFilter>("all");

  const filters: LibraryFilter[] = [
    "all",
    "favorites",
    // Emulated games by console rather than all together.
    ...SOURCES.filter((source) => source !== "emulator" && games.some((game) => game.source === source)),
    ...librarySystems(games).map(({ system }) => systemFilter(system)),
  ];
  const visible = sortGames(filterGames(games, filter), "name");

  return (
    <div ref={ref} className={styles.library}>
      <div className={styles.filters}>
        {filters.map((item) => (
          <button
            key={item}
            type="button"
            data-focusable
            className={styles.filter}
            aria-pressed={item === filter}
            onClick={() => setFilter(item)}
            {...hoverFocus}
          >
            {filterTitle(item, t)}
          </button>
        ))}
      </div>

      <VirtualGrid
        items={visible}
        getKey={(game) => game.id}
        minColumnWidth={tileWidth()}
        columnGap={28}
        rowGap={36}
        renderItem={(game, index) => (
          <LibraryTile game={game} autofocus={index === 0} onFocusGame={onFocusGame} onOpenGame={onOpenGame} />
        )}
      />
    </div>
  );
}
