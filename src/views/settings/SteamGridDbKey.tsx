import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Icon } from "../../components/Icon/Icon";
import { Spinner } from "../../components/Spinner/Spinner";
import { TextField } from "../../components/TextField/TextField";
import { STEAMGRIDDB_KEY_URL, importImages, openExternal, saveSteamGridDbKey } from "../../lib/backend";
import { useLibraryStore } from "../../state/library";
import styles from "./SteamGridDbKey.module.scss";

interface SteamGridDbKeyProps {
  connected: boolean;
  onChange: (connected: boolean) => void;
}

/** Enter, check and remove the SteamGridDB API key, and refresh artwork once connected. */
export function SteamGridDbKey({ connected, onChange }: SteamGridDbKeyProps) {
  const { t } = useTranslation();
  const progress = useLibraryStore((state) => state.imageProgress);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const save = () => {
    if (draft.trim() === "") return;
    setSaving(true);
    setError(null);
    saveSteamGridDbKey(draft)
      .then((saved) => {
        onChange(saved);
        setDraft("");
        if (saved) void importImages(false);
      })
      .catch((reason) => setError(reason === "invalid_key" ? t("settings.keyInvalid") : String(reason)))
      .finally(() => setSaving(false));
  };

  const remove = () => {
    saveSteamGridDbKey("")
      .then(onChange)
      .catch((reason) => setError(String(reason)));
  };

  return (
    <div className={styles.key}>
      {connected ? (
        <div className={styles.actions}>
          <span className={styles.badge}>
            <Icon name="check" size={14} />
            {t("settings.keyConnected")}
          </span>
          <Button variant="secondary" disabled={progress.running} onClick={() => void importImages(true)}>
            {t("settings.importNow")}
          </Button>
          <Button variant="ghost" onClick={remove}>
            {t("settings.keyRemove")}
          </Button>
        </div>
      ) : (
        <form
          className={styles.actions}
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <TextField
            className={styles.input}
            value={draft}
            onChange={setDraft}
            icon="eye"
            revealable
            invalid={error !== null}
            placeholder={t("settings.keyPlaceholder")}
            aria-label={t("settings.keyPlaceholder")}
            autoComplete="off"
            spellCheck={false}
          />
          <Button variant="primary" type="submit" disabled={draft.trim() === "" || saving}>
            {saving && <Spinner />}
            {saving ? t("settings.keySaving") : t("settings.keySave")}
          </Button>
        </form>
      )}

      {error && <p className={styles.error}>{error}</p>}

      {progress.running && (
        <p className={styles.progress}>
          <Spinner />
          {t("settings.importing", { done: progress.done, total: progress.total })}
        </p>
      )}

      {!connected && (
        <button type="button" data-focusable className={styles.link} onClick={() => void openExternal(STEAMGRIDDB_KEY_URL)}>
          {t("settings.keyGet")}
        </button>
      )}
    </div>
  );
}
