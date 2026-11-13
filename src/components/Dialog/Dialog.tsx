import type { ReactNode } from "react";
import { createPortal } from "react-dom";
import { cx } from "../../lib/cx";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import styles from "./Dialog.module.scss";

interface DialogProps {
  title: string;
  description?: string;
  onClose: () => void;
  /** While busy the dialog cannot be dismissed. */
  busy?: boolean;
  /** `lg` for long forms, `xl` for a fixed-size window whose content manages its own scrolling. */
  size?: "md" | "lg" | "xl";
  /** Shown before the title, e.g. the game's cover. */
  media?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

/** Modal window: dims the page, keeps navigation inside, closes on Escape or an outside click. */
export function Dialog({ title, description, onClose, busy = false, size = "md", media, children, footer }: DialogProps) {
  const ref = useAutofocus<HTMLDivElement>();

  useInputAction((action) => {
    if (action === "back" && !busy) {
      onClose();
      return true;
    }
  });

  // Rendered at the shell's root: inside a page, an animated (transformed) ancestor would
  // become the reference of `position: fixed` and cut the dialog.
  const host = document.querySelector("[data-overlay-root]") ?? document.body;

  return createPortal(
    <div className={styles.overlay} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <div ref={ref} className={cx(styles.dialog, size === "lg" && styles.large, size === "xl" && styles.xl)} role="dialog" aria-modal="true" aria-label={title} data-nav-trap>
        <header className={cx(media !== undefined && styles.withMedia)}>
          {media}
          <div>
            <h2>{title}</h2>
            {description && <p>{description}</p>}
          </div>
        </header>
        <div className={styles.body}>{children}</div>
        {footer && <footer>{footer}</footer>}
      </div>
    </div>,
    host,
  );
}
