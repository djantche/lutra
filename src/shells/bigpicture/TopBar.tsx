import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { hoverFocus } from "../../input/hoverFocus";
import { cx } from "../../lib/cx";
import styles from "./TopBar.module.scss";

/** Every page of Big Picture. */
export type Tab = "home" | "library" | "controllers" | "settings";
/** The pages in the top bar; the others (controllers, settings) live in the side menu. */
export const TABS: Tab[] = ["home", "library"];

function useClock(language: string): string {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(id);
  }, []);
  return new Intl.DateTimeFormat(language, { hour: "2-digit", minute: "2-digit" }).format(now);
}

export function TopBar({ tab, onTab, onMenu }: { tab: Tab; onTab: (tab: Tab) => void; onMenu: () => void }) {
  const { t, i18n } = useTranslation();
  const clock = useClock(i18n.language);

  return (
    <header className={styles.bar}>
      <div className={styles.left}>
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
        <time className={styles.clock}>{clock}</time>
      </div>
    </header>
  );
}
