import { cx } from "../../lib/cx";
import styles from "./ProgressBar.module.scss";

interface ProgressBarProps {
  /** 0 to 1, or null when the work has no measurable progress (it then slides). */
  value: number | null;
  ariaLabel: string;
  /** The work failed: the bar turns red where it stopped. */
  failed?: boolean;
}

/** A rounded bar that fills as real work advances. Only `transform` moves. */
export function ProgressBar({ value, ariaLabel, failed = false }: ProgressBarProps) {
  return (
    <div
      className={cx(styles.track, value === null && styles.indeterminate, failed && styles.failed)}
      role="progressbar"
      aria-label={ariaLabel}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value === null ? undefined : Math.round(value * 100)}
    >
      <div className={styles.fill} style={value === null ? undefined : { transform: `scaleX(${value})` }} />
    </div>
  );
}
