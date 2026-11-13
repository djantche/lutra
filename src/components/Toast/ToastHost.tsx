import { useToastStore } from "../../state/toasts";
import { Icon } from "../Icon/Icon";
import styles from "./ToastHost.module.scss";

/** Stack of short messages in the corner of the window. */
export function ToastHost() {
  const toasts = useToastStore((state) => state.toasts);
  const dismiss = useToastStore((state) => state.dismiss);

  return (
    <div className={styles.host} role="log" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={styles.toast} data-kind={toast.kind}>
          <span>{toast.text}</span>
          <button type="button" className={styles.close} aria-label="×" onClick={() => dismiss(toast.id)}>
            <Icon name="close" size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
