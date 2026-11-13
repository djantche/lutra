import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import styles from "./FilePicker.module.scss";

interface FilePickerProps {
  /** Currently chosen path, empty when nothing is chosen. */
  value: string;
  placeholder: string;
  onPick: () => void;
  autofocus?: boolean;
}

/** Shows the chosen path and opens the file chooser (provided by the caller) when pressed. */
export function FilePicker({ value, placeholder, onPick, autofocus = false }: FilePickerProps) {
  return (
    <button
      type="button"
      data-focusable
      data-autofocus={autofocus ? "" : undefined}
      className={styles.picker}
      onClick={onPick}
    >
      <Icon name="folder" size={16} />
      <span className={cx(styles.text, value === "" && styles.empty)}>{value || placeholder}</span>
    </button>
  );
}
