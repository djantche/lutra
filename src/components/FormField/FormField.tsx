import type { ReactNode } from "react";
import styles from "./FormField.module.scss";

/** Label, control and optional error message, stacked. Clicking the label focuses the control. */
interface FormFieldProps {
  label: string;
  /** Short help shown under the control. */
  hint?: string;
  error?: string | null;
  children: ReactNode;
}

export function FormField({ label, hint, error, children }: FormFieldProps) {
  return (
    <label className={styles.field}>
      <span className={styles.label}>{label}</span>
      {children}
      {hint && !error && <span className={styles.hint}>{hint}</span>}
      {error && <span className={styles.error}>{error}</span>}
    </label>
  );
}
