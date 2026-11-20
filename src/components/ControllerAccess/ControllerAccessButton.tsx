import { useState } from "react";
import { useTranslation } from "react-i18next";
import { setupControllerAccess } from "../../lib/controllers";
import { Button } from "../Button/Button";
import { Spinner } from "../Spinner/Spinner";
import styles from "./ControllerAccess.module.scss";

/**
 * Sets controllers up in one click: the system asks for the password, Lutra does the rest (see
 * `backend/src/access.rs`). Nobody has to type a command.
 */
export function ControllerAccessButton({ onDone, autoFocus = false }: { onDone: () => void; autoFocus?: boolean }) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = () => {
    setBusy(true);
    setError(null);
    setupControllerAccess()
      .then(onDone)
      .catch((reason) => {
        const code = String(reason);
        // A dismissed password dialog is an answer, not a failure.
        setError(code === "cancelled" ? null : t(`controllers.access.error.${["no_pkexec", "no_authentication", "not_applied"].includes(code) ? code : "other"}`, { reason: code }));
        setBusy(false);
      });
  };

  return (
    <div className={styles.setup}>
      <Button variant="primary" icon="gamepad" disabled={busy} data-autofocus={autoFocus ? "" : undefined} onClick={run}>
        {busy && <Spinner />}
        {busy ? t("controllers.access.waiting") : t("controllers.access.button")}
      </Button>
      {error && <p className={styles.error}>{error}</p>}
    </div>
  );
}
