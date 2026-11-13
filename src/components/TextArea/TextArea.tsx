import type { TextareaHTMLAttributes } from "react";
import { cx } from "../../lib/cx";
import styles from "./TextArea.module.scss";

interface TextAreaProps extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "value" | "onChange"> {
  value: string;
  onChange: (value: string) => void;
  className?: string;
}

/** Multi-line text in the same rounded frame as `TextField`: the frame lights up, not the inner box. */
export function TextArea({ value, onChange, className, rows = 4, ...input }: TextAreaProps) {
  return (
    <div className={cx(styles.field, className)}>
      <textarea
        {...input}
        data-focusable
        rows={rows}
        className={styles.input}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}
