import type { ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./SegmentedControl.module.scss";

export interface Segment<T extends string> {
  value: T;
  label?: ReactNode;
  icon?: IconName;
  /** Required for icon-only segments. */
  ariaLabel?: string;
}

interface SegmentedControlProps<T extends string> {
  segments: Segment<T>[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
  /** Marks the selected segment as the one to focus when its screen opens. */
  autofocus?: boolean;
  size?: "sm" | "md";
}

/** A row of mutually exclusive choices. */
export function SegmentedControl<T extends string>({
  segments,
  value,
  onChange,
  ariaLabel,
  autofocus = false,
  size = "md",
}: SegmentedControlProps<T>) {
  return (
    <div className={cx(styles.control, styles[size])} role="group" aria-label={ariaLabel}>
      {segments.map((segment) => (
        <button
          key={segment.value}
          type="button"
          data-focusable
          data-autofocus={autofocus && segment.value === value ? "" : undefined}
          className={styles.segment}
          aria-pressed={segment.value === value}
          aria-label={segment.ariaLabel}
          title={segment.ariaLabel}
          onClick={() => onChange(segment.value)}
        >
          {segment.icon && <Icon name={segment.icon} size={16} />}
          {segment.label}
        </button>
      ))}
    </div>
  );
}
