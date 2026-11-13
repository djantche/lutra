import type { ReactNode } from "react";
import { cx } from "../../lib/cx";
import styles from "./SettingsGroup.module.scss";

/** A titled card holding related settings rows. Sized in em so each shell can scale it. */
export function SettingsGroup({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={styles.group}>
      <h2 className={styles.title}>{title}</h2>
      <div className={styles.card}>{children}</div>
    </section>
  );
}

interface SettingsRowProps {
  title: string;
  description?: string;
  /** Puts the control under the text instead of beside it. */
  stacked?: boolean;
  children?: ReactNode;
}

export function SettingsRow({ title, description, stacked = false, children }: SettingsRowProps) {
  return (
    <div className={cx(styles.row, stacked && styles.stacked)} data-nav-row tabIndex={-1}>
      <div className={styles.text}>
        <h3>{title}</h3>
        {description && <p>{description}</p>}
      </div>
      {children && <div className={styles.control}>{children}</div>}
    </div>
  );
}
