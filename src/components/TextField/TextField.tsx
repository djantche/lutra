import { useState, type InputHTMLAttributes } from "react";
import { useTranslation } from "react-i18next";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./TextField.module.scss";

interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange" | "size"> {
  value: string;
  onChange: (value: string) => void;
  /** Leading icon. */
  icon?: IconName;
  /** Shows a button that empties the field. */
  clearable?: boolean;
  /** For secrets: adds a button that toggles the text visibility. */
  revealable?: boolean;
  invalid?: boolean;
  /** Lets a parent shape the outer box (width, radius). */
  className?: string;
}

export function TextField({
  value,
  onChange,
  icon,
  clearable = false,
  revealable = false,
  invalid = false,
  type = "text",
  className,
  ...input
}: TextFieldProps) {
  const { t } = useTranslation();
  const [revealed, setRevealed] = useState(false);

  return (
    <div className={cx(styles.field, invalid && styles.invalid, className)}>
      {icon && <Icon name={icon} size={16} />}
      <input
        {...input}
        data-focusable
        className={styles.input}
        type={revealable && !revealed ? "password" : type}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {clearable && value !== "" && (
        <button type="button" className={styles.action} aria-label={t("common.clear")} onClick={() => onChange("")}>
          <Icon name="close" size={13} />
        </button>
      )}
      {revealable && (
        <button
          type="button"
          className={styles.action}
          aria-label={revealed ? t("common.hide") : t("common.show")}
          onClick={() => setRevealed((current) => !current)}
        >
          <Icon name={revealed ? "eyeOff" : "eye"} size={15} />
        </button>
      )}
    </div>
  );
}
