import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./ChoiceCards.module.scss";

export interface ChoiceOption<T extends string> {
  value: T;
  title: string;
  description?: string;
  icon?: IconName;
  /** A short tag next to the title ("Default"). */
  badge?: string;
  disabled?: boolean;
}

interface ChoiceCardsProps<T extends string> {
  options: ChoiceOption<T>[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
  /** `grid` for a few big choices side by side, `list` for a column of rows. */
  layout?: "grid" | "list";
  /** With four choices, `grid` makes two rows of two instead of three and a lonely one. */
  wide?: boolean;
  /** The selected card is the one to focus when its screen opens. */
  autofocus?: boolean;
}

/** Mutually exclusive choices drawn as cards: the whole card is the control. */
export function ChoiceCards<T extends string>({
  options,
  value,
  onChange,
  ariaLabel,
  layout = "grid",
  wide = false,
  autofocus = false,
}: ChoiceCardsProps<T>) {
  return (
    <div className={cx(styles.cards, styles[layout], wide && styles.wide)} role="radiogroup" aria-label={ariaLabel}>
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            data-focusable
            data-autofocus={autofocus && selected ? "" : undefined}
            className={cx(styles.card, selected && styles.selected)}
            disabled={option.disabled}
            onClick={() => onChange(option.value)}
          >
            {option.icon && (
              <span className={styles.icon} aria-hidden="true">
                <Icon name={option.icon} size={22} />
              </span>
            )}
            <span className={styles.text}>
              <span className={styles.title}>
                {option.title}
                {option.badge && <span className={styles.badge}>{option.badge}</span>}
              </span>
              {option.description && <small>{option.description}</small>}
            </span>
            <span className={styles.mark} aria-hidden="true">
              {selected && <Icon name="check" size={14} />}
            </span>
          </button>
        );
      })}
    </div>
  );
}
