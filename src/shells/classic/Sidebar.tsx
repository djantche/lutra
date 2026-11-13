import { useTranslation } from "react-i18next";
import { Icon, type IconName } from "../../components/Icon/Icon";
import { cx } from "../../lib/cx";
import { filterGames, type LibraryFilter } from "../../lib/library";
import { SOURCES, SOURCE_COLOR } from "../../lib/sources";
import { useLibrary } from "../../state/library";
import { useUi } from "../../state/ui";
import type { Page } from "./navigation";
import styles from "./Sidebar.module.scss";

interface NavItemProps {
  label: string;
  active: boolean;
  onClick: () => void;
  icon?: IconName;
  dotColor?: string;
  count?: number;
}

function NavItem({ label, active, onClick, icon, dotColor, count }: NavItemProps) {
  return (
    <button
      type="button"
      data-focusable
      className={cx(styles.item, active && styles.active)}
      aria-current={active ? "page" : undefined}
      onClick={onClick}
    >
      {icon && <Icon name={icon} size={18} />}
      {dotColor && <span className={styles.dot} style={{ background: dotColor }} />}
      <span className={styles.label}>{label}</span>
      {count !== undefined && <span className={styles.count}>{count}</span>}
    </button>
  );
}

export function Sidebar({ page, onNavigate }: { page: Page; onNavigate: (page: Page) => void }) {
  const { t } = useTranslation();
  const { games } = useLibrary();
  const { setMode, setAddGameOpen } = useUi();

  const isLibrary = (filter: LibraryFilter) => page.name === "library" && page.filter === filter;
  const library = (filter: LibraryFilter) => () => onNavigate({ name: "library", filter });
  const sources = SOURCES.filter((source) => games.some((game) => game.source === source));

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand} data-tauri-drag-region>
        <img src="/lutra.svg" alt="" width={28} height={28} />
        <span>Lutra</span>
      </div>

      <nav className={styles.nav}>
        <NavItem
          label={t("nav.home")}
          icon="home"
          active={page.name === "home"}
          onClick={() => onNavigate({ name: "home" })}
        />
        <NavItem label={t("nav.allGames")} icon="grid" active={isLibrary("all")} onClick={library("all")} />
        <NavItem
          label={t("nav.favorites")}
          icon="star"
          count={filterGames(games, "favorites").length}
          active={isLibrary("favorites")}
          onClick={library("favorites")}
        />
        <NavItem label={t("nav.recent")} icon="clock" active={isLibrary("recent")} onClick={library("recent")} />

        <h2 className={styles.sectionTitle}>{t("nav.sources")}</h2>
        {sources.map((source) => (
          <NavItem
            key={source}
            label={t(`sources.${source}`)}
            dotColor={SOURCE_COLOR[source]}
            count={filterGames(games, source).length}
            active={isLibrary(source)}
            onClick={library(source)}
          />
        ))}
      </nav>

      <div className={styles.footer}>
        <NavItem label={t("nav.addGame")} icon="plus" active={false} onClick={() => setAddGameOpen(true)} />
        <NavItem
          label={t("nav.controllers")}
          icon="gamepad"
          active={page.name === "controllers"}
          onClick={() => onNavigate({ name: "controllers" })}
        />
        <NavItem
          label={t("nav.settings")}
          icon="settings"
          active={page.name === "settings"}
          onClick={() => onNavigate({ name: "settings" })}
        />
        <button type="button" data-focusable className={styles.bigPicture} onClick={() => setMode("bigpicture")}>
          <span className={styles.bigPictureIcon}>
            <Icon name="gamepad" size={20} />
          </span>
          <span className={styles.bigPictureText}>
            <strong>{t("nav.bigPicture")}</strong>
            <small>{t("nav.bigPictureHint")}</small>
          </span>
          <Icon name="chevronRight" size={16} />
        </button>
      </div>
    </aside>
  );
}
