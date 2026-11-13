import { useLayoutEffect, useRef } from "react";
import { hoverFocus } from "../../input/hoverFocus";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./TabBar.module.scss";

export interface TabBarItem<T extends string> {
  value: T;
  label: string;
  icon: IconName;
  /** Small marker, e.g. to flag a section with unsaved changes. */
  badge?: boolean;
}

interface TabBarProps<T extends string> {
  items: TabBarItem<T>[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
  /** Larger tabs with white focus, for Big Picture. */
  large?: boolean;
}

/**
 * A row of sections, with the content under it: left and right move between tabs, down goes into
 * the content. Used instead of a side list, which is awkward to cross with a keyboard or a pad.
 */
export function TabBar<T extends string>({ items, value, onChange, ariaLabel, large = false }: TabBarProps<T>) {
  const bar = useRef<HTMLElement>(null);

  // The underline slides to the current tab: its place is measured, never guessed.
  useLayoutEffect(() => {
    const element = bar.current;
    if (!element) return;
    const place = () => {
      const current = element.querySelector<HTMLElement>("[aria-current='page']");
      if (!current) return;
      element.style.setProperty("--tab-x", `${current.offsetLeft}px`);
      element.style.setProperty("--tab-w", `${current.offsetWidth}px`);
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(element);
    return () => observer.disconnect();
  }, [value, items.length]);

  return (
    <nav ref={bar} className={cx(styles.bar, large && styles.large)} aria-label={ariaLabel}>
      <i className={styles.indicator} aria-hidden="true" />
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          data-focusable
          data-autofocus={item.value === value ? "" : undefined}
          className={cx(styles.tab, item.value === value && styles.active)}
          aria-current={item.value === value ? "page" : undefined}
          onClick={() => onChange(item.value)}
          {...(large ? hoverFocus : {})}
        >
          <Icon name={item.icon} size={large ? 22 : 17} />
          <span>{item.label}</span>
          {item.badge && <i className={styles.badge} />}
        </button>
      ))}
    </nav>
  );
}
