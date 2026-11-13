import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Icon, type IconName } from "../../components/Icon/Icon";
import { hoverFocus } from "../../input/hoverFocus";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import { cx } from "../../lib/cx";
import type { Tab } from "./TopBar";
import styles from "./BigPictureMenu.module.scss";

interface MenuItem {
  tab: Tab;
  icon: IconName;
}

const ITEMS: MenuItem[] = [
  { tab: "home", icon: "home" },
  { tab: "library", icon: "grid" },
  { tab: "controllers", icon: "gamepad" },
  { tab: "settings", icon: "settings" },
];

interface BigPictureMenuProps {
  tab: Tab;
  onTab: (tab: Tab) => void;
  onClose: () => void;
  onExit: () => void;
  /** Hides Lutra in the notification area. */
  onMinimize: () => void;
  onQuit: () => void;
}

/**
 * The side menu of Big Picture, opened with the controller's menu button: where you go (home,
 * library) and the settings (controllers, general), plus the way back to the desktop.
 */
export function BigPictureMenu({ tab, onTab, onClose, onExit, onMinimize, onQuit }: BigPictureMenuProps) {
  const { t } = useTranslation();
  const ref = useAutofocus<HTMLDivElement>();

  // The tile or button that was focused gets the focus back when the menu closes.
  useEffect(() => {
    const opener = document.activeElement;
    return () => {
      if (opener instanceof HTMLElement) opener.focus({ preventScroll: true });
    };
  }, []);

  useInputAction((action) => {
    if (action === "back" || action === "menu") {
      onClose();
      return true;
    }
  });

  return (
    <div className={styles.root} role="dialog" aria-modal="true" aria-label={t("bigPicture.menu")} data-nav-trap ref={ref}>
      <button type="button" className={styles.scrim} tabIndex={-1} aria-label={t("common.back")} onClick={onClose} />
      <nav className={styles.panel}>
        <div className={styles.brand}>
          <img src="/lutra.svg" alt="" width={40} height={40} />
          <strong>Lutra</strong>
        </div>

        <ul className={styles.list}>
          {ITEMS.map((item) => (
            <li key={item.tab}>
              <button
                type="button"
                data-focusable
                data-autofocus={item.tab === tab ? "" : undefined}
                className={cx(styles.item, item.tab === tab && styles.current)}
                aria-current={item.tab === tab ? "page" : undefined}
                onClick={() => {
                  onTab(item.tab);
                  onClose();
                }}
                {...hoverFocus}
              >
                <Icon name={item.icon} size={26} />
                {t(`nav.${item.tab}`)}
              </button>
            </li>
          ))}
        </ul>

        <button type="button" data-focusable className={cx(styles.item, styles.exit)} onClick={onExit} {...hoverFocus}>
          <Icon name="monitor" size={26} />
          {t("bigPicture.exit")}
        </button>
        <button type="button" data-focusable className={styles.item} onClick={onMinimize} {...hoverFocus}>
          <Icon name="minimize" size={26} />
          {t("bigPicture.minimize")}
        </button>
        <button type="button" data-focusable className={cx(styles.item, styles.last)} onClick={onQuit} {...hoverFocus}>
          <Icon name="power" size={26} />
          {t("bigPicture.quit")}
        </button>
      </nav>
    </div>
  );
}
