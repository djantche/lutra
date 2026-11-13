import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./SideNav.module.scss";

export interface SideNavItem<T extends string> {
  value: T;
  label: string;
  icon: IconName;
  /** Small marker, e.g. to flag a section with unsaved changes. */
  badge?: boolean;
}

interface SideNavProps<T extends string> {
  items: SideNavItem<T>[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
}

/** Vertical list of sections, for windows with more content than tabs can hold. */
export function SideNav<T extends string>({ items, value, onChange, ariaLabel }: SideNavProps<T>) {
  return (
    <nav className={styles.nav} aria-label={ariaLabel}>
      {items.map((item) => (
        <button
          key={item.value}
          type="button"
          data-focusable
          data-autofocus={item.value === value ? "" : undefined}
          className={cx(styles.item, item.value === value && styles.active)}
          aria-current={item.value === value ? "page" : undefined}
          onClick={() => onChange(item.value)}
        >
          <Icon name={item.icon} size={17} />
          <span>{item.label}</span>
          {item.badge && <i className={styles.badge} />}
        </button>
      ))}
    </nav>
  );
}
