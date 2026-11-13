import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { useRef } from "react";
import { hoverFocus } from "../../input/hoverFocus";
import { cx } from "../../lib/cx";
import { useInputAction } from "../../input/useInputAction";
import { StatusCluster } from "./StatusCluster";
import styles from "./TopBar.module.scss";

/** Every page of Big Picture. */
export type Tab = "home" | "library" | "controllers" | "settings";
/** The pages in the top bar; the others (controllers, settings) live in the side menu. */
export const TABS: Tab[] = ["home", "library"];

interface TopBarProps {
  tab: Tab;
  onTab: (tab: Tab) => void;
  onMenu: () => void;
  onSearch: () => void;
}

export function TopBar({ tab, onTab, onMenu, onSearch }: TopBarProps) {
  const { t } = useTranslation();
  const bar = useRef<HTMLElement>(null);

  // Nothing is above the top bar: going up from it opens the search, as on a console.
  useInputAction((action) => {
    if (action !== "up" || !bar.current?.contains(document.activeElement)) return;
    onSearch();
    return true;
  });

  return (
    <header ref={bar} className={styles.bar}>
      <div className={styles.left}>
        <div className={styles.buttons}>
          <button
            type="button"
            data-focusable
            className={styles.menu}
            aria-label={t("bigPicture.menu")}
            onClick={onMenu}
            {...hoverFocus}
          >
            <Icon name="menu" size={24} />
          </button>
          <button
            type="button"
            data-focusable
            className={styles.menu}
            aria-label={t("bigPicture.searchOpen")}
            onClick={onSearch}
            {...hoverFocus}
          >
            <Icon name="search" size={24} />
          </button>
        </div>
        <nav className={styles.tabs}>
          {TABS.map((item) => (
            <button
              key={item}
              type="button"
              data-focusable
              className={cx(styles.tab, item === tab && styles.active)}
              aria-current={item === tab ? "page" : undefined}
              onClick={() => onTab(item)}
              {...hoverFocus}
            >
              {t(`nav.${item}`)}
            </button>
          ))}
        </nav>
      </div>

      <div className={styles.right}>
        <StatusCluster />
      </div>
    </header>
  );
}
