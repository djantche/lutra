import { playSound } from "../../lib/sounds";
import styles from "./Switch.module.scss";

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  /** Accessible name; the visible label usually lives next to the switch. */
  ariaLabel: string;
  disabled?: boolean;
}

export function Switch({ checked, onChange, ariaLabel, disabled = false }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      data-focusable
      className={styles.track}
      disabled={disabled}
      onClick={() => {
        playSound(checked ? "off" : "on");
        onChange(!checked);
      }}
    >
      <span className={styles.thumb} />
    </button>
  );
}
