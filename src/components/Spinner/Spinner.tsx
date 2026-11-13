import styles from "./Spinner.module.scss";

/** Inline loading indicator that takes the text color and size of its context. */
export function Spinner() {
  return <span className={styles.spinner} aria-hidden="true" />;
}
