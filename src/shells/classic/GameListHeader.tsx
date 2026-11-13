import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { cx } from "../../lib/cx";
import type { SortKey } from "../../lib/library";
import rows from "./GameRow.module.scss";
import styles from "./GameListHeader.module.scss";

interface GameListHeaderProps {
  sort: SortKey;
  onSort: (sort: SortKey) => void;
}

/** Column titles of the list view; clicking one sorts by it. */
export function GameListHeader({ sort, onSort }: GameListHeaderProps) {
  const { t } = useTranslation();

  const column = (key: SortKey, label: string) => (
    <button
      type="button"
      className={cx(styles.column, sort === key && styles.active)}
      aria-pressed={sort === key}
      onClick={() => onSort(key)}
    >
      {label}
      {sort === key && <Icon name="chevronDown" size={14} />}
    </button>
  );

  return (
    <div className={cx(rows.columns, styles.header)} role="presentation">
      <span />
      {column("name", t("library.sortName"))}
      <span className={styles.static}>{t("game.source")}</span>
      {column("playtime", t("library.sortPlaytime"))}
      {column("lastPlayed", t("library.sortLastPlayed"))}
      <span />
    </div>
  );
}
