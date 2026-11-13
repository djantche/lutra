import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { removablePrefix } from "../../lib/backend";
import { formatBytes } from "../../lib/format";
import { useLibraryStore } from "../../state/library";
import { reportError } from "../../state/toasts";
import type { Game } from "../../types/game";
import { Button } from "../Button/Button";
import { Dialog } from "../Dialog/Dialog";
import { Spinner } from "../Spinner/Spinner";
import { Switch } from "../Switch/Switch";
import styles from "./RemoveGameDialog.module.scss";

/**
 * Removes a game from the library, and on request everything Lutra keeps for it: its Wine
 * prefix (save games and anything installed in it included), logs and controller profile.
 */
export function RemoveGameDialog({ game, onClose }: { game: Game; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const removeGame = useLibraryStore((state) => state.removeGame);
  // undefined: still measuring; null: nothing Lutra may delete besides the entry.
  const [prefix, setPrefix] = useState<{ path: string; bytes: number } | null | undefined>(undefined);
  const [everything, setEverything] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    removablePrefix(game.id)
      .then(setPrefix)
      .catch(() => setPrefix(null));
  }, [game.id]);

  const confirm = () => {
    setBusy(true);
    removeGame(game.id, everything)
      .then(onClose)
      .catch((error) => {
        reportError(t("errors.remove"), error);
        setBusy(false);
      });
  };

  return (
    <Dialog
      title={t("game.removeTitle", { title: game.title })}
      description={everything ? t("remove.everythingHint") : t("game.removeHint")}
      onClose={onClose}
      busy={busy}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" className={everything ? styles.danger : undefined} disabled={busy} onClick={confirm}>
            {busy && <Spinner />}
            {everything ? t("remove.everything") : t("game.remove")}
          </Button>
        </>
      }
    >
      {prefix === undefined && (
        <p className={styles.measuring}>
          <Spinner /> {t("remove.measuring")}
        </p>
      )}
      {prefix && (
        <div className={styles.option} data-checked={everything || undefined}>
          <div className={styles.text}>
            <strong>{t("remove.prefix", { size: formatBytes(prefix.bytes, i18n.language) })}</strong>
            <small>{prefix.path}</small>
            <small>{t("remove.prefixHint")}</small>
          </div>
          <Switch ariaLabel={t("remove.prefix", { size: "" })} checked={everything} onChange={setEverything} />
        </div>
      )}
    </Dialog>
  );
}
