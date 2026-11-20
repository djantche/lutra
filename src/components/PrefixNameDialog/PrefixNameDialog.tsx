import { useState } from "react";
import { useTranslation } from "react-i18next";
import { renamePrefix, type PrefixInfo } from "../../lib/backend";
import { PREFIX_NAME_MAX, prefixTitle } from "../../lib/prefixes";
import { reportError } from "../../state/toasts";
import { Button } from "../Button/Button";
import { Dialog } from "../Dialog/Dialog";
import { FormField } from "../FormField/FormField";
import { GameChips } from "../PrefixCard/PrefixCard";
import { Spinner } from "../Spinner/Spinner";
import { TextField } from "../TextField/TextField";
import styles from "./PrefixNameDialog.module.scss";

interface PrefixNameDialogProps {
  prefix: PrefixInfo;
  onClose: () => void;
  /** The name was saved (or cleared): the lists showing prefixes should be read again. */
  onSaved: () => void;
}

/** Gives a prefix a name of the user's own, or takes it back to the automatic one. */
export function PrefixNameDialog({ prefix, onClose, onSaved }: PrefixNameDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState(prefix.customName ?? "");
  const [busy, setBusy] = useState(false);

  // What the prefix is called when the field is empty.
  const automatic = prefixTitle({ ...prefix, customName: null }, t);

  const save = (value: string) => {
    setBusy(true);
    renamePrefix(prefix.path, value)
      .then(() => {
        onSaved();
        onClose();
      })
      .catch((error) => {
        reportError(t("prefix.renameError"), error);
        setBusy(false);
      });
  };

  const unchanged = name.trim() === (prefix.customName ?? "");

  return (
    <Dialog
      title={t("prefix.nameTitle")}
      description={t("prefix.nameHint")}
      onClose={onClose}
      busy={busy}
      footer={
        <>
          {prefix.customName && (
            <Button variant="ghost" className={styles.reset} disabled={busy} onClick={() => save("")}>
              {t("prefix.useAutomatic")}
            </Button>
          )}
          <Button variant="ghost" disabled={busy} onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button variant="primary" disabled={busy || unchanged} onClick={() => save(name)}>
            {busy && <Spinner />}
            {t("common.save")}
          </Button>
        </>
      }
    >
      <FormField label={t("prefix.nameLabel")} hint={t("prefix.nameCount", { count: name.length, max: PREFIX_NAME_MAX })}>
        <TextField
          value={name}
          onChange={setName}
          maxLength={PREFIX_NAME_MAX}
          placeholder={automatic}
          clearable
          data-autofocus=""
          onKeyDown={(event) => {
            if (event.key === "Enter" && !busy && !unchanged) save(name);
          }}
        />
      </FormField>

      <section className={styles.games}>
        <h3>{t("prefix.usedBy")}</h3>
        {prefix.games.length > 0 ? <GameChips games={prefix.games} /> : <p>{t("prefix.noGames")}</p>}
      </section>

      <p className={styles.path} title={prefix.path}>
        {prefix.path}
      </p>
    </Dialog>
  );
}
