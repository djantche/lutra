import { useEffect, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { useContextMenuStore } from "../../state/contextMenu";
import { Icon } from "../Icon/Icon";
import styles from "./ContextMenuHost.module.scss";

const EDGE = 8;

/** The one right-click menu of the window. Items come from whoever opened it. */
export function ContextMenuHost() {
  const menu = useContextMenuStore((state) => state.menu);
  const close = useContextMenuStore((state) => state.close);
  const root = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: 0, top: 0 });

  // Keeps the menu fully on screen, whichever corner it was opened from.
  useLayoutEffect(() => {
    if (!menu || !root.current) return;
    const { width, height } = root.current.getBoundingClientRect();
    setPosition({
      left: Math.max(EDGE, Math.min(menu.x, window.innerWidth - width - EDGE)),
      top: Math.max(EDGE, Math.min(menu.y, window.innerHeight - height - EDGE)),
    });
  }, [menu]);

  // Focus moves into the menu, and comes back to where it was when the menu closes.
  useEffect(() => {
    if (!menu) return;
    const opener = document.activeElement;
    root.current?.querySelector<HTMLElement>("[role=menuitem]:not(:disabled)")?.focus();
    return () => {
      if (opener instanceof HTMLElement && document.contains(opener)) opener.focus({ preventScroll: true });
    };
  }, [menu]);

  useEffect(() => {
    if (!menu) return;
    const dismiss = (event: Event) => {
      if (event instanceof PointerEvent && root.current?.contains(event.target as Node)) return;
      close();
    };
    window.addEventListener("pointerdown", dismiss, true);
    window.addEventListener("blur", dismiss);
    window.addEventListener("resize", dismiss);
    window.addEventListener("wheel", dismiss, { passive: true });
    return () => {
      window.removeEventListener("pointerdown", dismiss, true);
      window.removeEventListener("blur", dismiss);
      window.removeEventListener("resize", dismiss);
      window.removeEventListener("wheel", dismiss);
    };
  }, [menu, close]);

  if (!menu) return null;

  const onKeyDown = (event: KeyboardEvent) => {
    const items = Array.from(root.current?.querySelectorAll<HTMLElement>("[role=menuitem]:not(:disabled)") ?? []);
    const index = items.indexOf(document.activeElement as HTMLElement);
    const focus = (next: number) => {
      event.preventDefault();
      items[(next + items.length) % items.length]?.focus();
    };

    switch (event.key) {
      case "ArrowDown":
        return focus(index + 1);
      case "ArrowUp":
        return focus(index - 1);
      case "Home":
        return focus(0);
      case "End":
        return focus(items.length - 1);
      case "Escape":
      case "Tab":
        event.preventDefault();
        return close();
    }
  };

  return (
    <div
      ref={root}
      className={styles.menu}
      role="menu"
      style={position}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => event.preventDefault()}
    >
      {menu.items.map((item, index) =>
        item.type === "separator" ? (
          <div key={index} className={styles.separator} role="separator" />
        ) : (
          <button
            key={index}
            type="button"
            role="menuitem"
            className={cx(styles.item, item.danger && styles.danger)}
            disabled={item.disabled}
            onClick={() => {
              close();
              item.onSelect();
            }}
          >
            <span className={styles.icon}>{item.icon && <Icon name={item.icon} size={16} />}</span>
            {item.label}
          </button>
        ),
      )}
    </div>
  );
}
