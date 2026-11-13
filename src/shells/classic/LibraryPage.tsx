import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyLibrary } from "../../components/EmptyLibrary/EmptyLibrary";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { Icon } from "../../components/Icon/Icon";
import { VirtualGrid } from "../../components/VirtualGrid/VirtualGrid";
import { filterGames, filterTitle, sortGames, type LibraryFilter, type SortKey } from "../../lib/library";
import { SOURCES } from "../../lib/sources";
import { useLibrary } from "../../state/library";
import { GameCard } from "./GameCard";
import { GameListHeader } from "./GameListHeader";
import { GameRow } from "./GameRow";
import styles from "./LibraryPage.module.scss";

type Layout = "grid" | "list";

interface LibraryPageProps {
  filter: LibraryFilter;
  query?: string;
  onOpenGame: (id: string) => void;
}

export function LibraryPage({ filter, query = "", onOpenGame }: LibraryPageProps) {
  const { t } = useTranslation();
  const { games } = useLibrary();
  const [sort, setSort] = useState<SortKey>(filter === "recent" ? "lastPlayed" : "name");
  const [layout, setLayout] = useState<Layout>("grid");

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const matching = filterGames(games, filter).filter(
      (game) => game.title.toLowerCase().includes(needle) || game.developer?.toLowerCase().includes(needle),
    );
    return sortGames(matching, sort);
  }, [games, filter, query, sort]);

  // Inside a source page, repeating the source on every game is noise.
  const inSource = !query.trim() && SOURCES.some((source) => source === filter);
  const title = query.trim() ? t("library.results", { query: query.trim() }) : filterTitle(filter, t);
  const sorts: { value: SortKey; label: string }[] = [
    { value: "name", label: t("library.sortName") },
    { value: "lastPlayed", label: t("library.sortLastPlayed") },
    { value: "playtime", label: t("library.sortPlaytime") },
  ];

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <h1>{title}</h1>
          <p>{t("library.count", { count: visible.length })}</p>
        </div>

        <div className={styles.tools}>
          <SegmentedControl
            size="sm"
            ariaLabel={t("library.sortBy")}
            segments={sorts}
            value={sort}
            onChange={setSort}
          />
          <SegmentedControl
            size="sm"
            ariaLabel={t("library.layout")}
            segments={[
              { value: "grid", icon: "grid", ariaLabel: t("library.viewGrid") },
              { value: "list", icon: "list", ariaLabel: t("library.viewList") },
            ]}
            value={layout}
            onChange={setLayout}
          />
        </div>
      </header>

      {games.length === 0 ? (
        <EmptyLibrary />
      ) : visible.length === 0 ? (
        <div className={styles.empty}>
          <Icon name="search" size={28} />
          <h2>{t("library.empty")}</h2>
          <p>{t("library.emptyHint")}</p>
        </div>
      ) : layout === "grid" ? (
        <VirtualGrid
          items={visible}
          getKey={(game) => game.id}
          minColumnWidth={170}
          columnGap={20}
          rowGap={32}
          renderItem={(game) => <GameCard game={game} onOpen={onOpenGame} showSource={!inSource} />}
        />
      ) : (
        <div className={styles.list}>
          <GameListHeader sort={sort} onSort={setSort} />
          <VirtualGrid
            items={visible}
            getKey={(game) => game.id}
            minColumnWidth={0}
            columnGap={0}
            rowGap={2}
            renderItem={(game) => <GameRow game={game} onOpen={onOpenGame} showSource={!inSource} />}
          />
        </div>
      )}
    </div>
  );
}
