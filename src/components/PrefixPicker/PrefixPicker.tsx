import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { checkPrefix, listPrefixes, pickFolder, type PrefixInfo, type Runner } from "../../lib/backend";
import { Select, type SelectOption } from "../Select/Select";
import styles from "./PrefixPicker.module.scss";

/** Value that means "create a new prefix for this game". */
export const NEW_PREFIX = "";
const BROWSE = "\u0000browse";

interface PrefixPickerProps {
  /** The runner decides which prefixes can be used: Wine and Proton ones are not interchangeable. */
  runnerKind: Runner["kind"];
  /** `NEW_PREFIX`, or the folder of an existing prefix. */
  value: string;
  onChange: (path: string) => void;
  /** Offers "new prefix": for a game that has none yet. */
  allowNew?: boolean;
  ariaLabel: string;
}

/**
 * Chooses a game's Wine prefix from the ones that exist. A prefix the runner cannot use is shown
 * greyed out with the reason; a folder picked by hand is checked by the backend before it is kept.
 */
export function PrefixPicker({ runnerKind, value, onChange, allowNew = false, ariaLabel }: PrefixPickerProps) {
  const { t } = useTranslation();
  const [known, setKnown] = useState<PrefixInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listPrefixes()
      .then(setKnown)
      .catch(() => setKnown([]));
  }, []);

  // A prefix the runner cannot use never stays selected: fall back to a new one.
  useEffect(() => {
    const current = known.find((prefix) => prefix.path === value);
    if (allowNew && current?.kind && current.kind !== runnerKind) onChange(NEW_PREFIX);
  }, [known, runnerKind, value, allowNew, onChange]);

  const options = useMemo(() => {
    const list: SelectOption<string>[] = [];
    if (allowNew) list.push({ value: NEW_PREFIX, label: t("prefix.new"), hint: t("prefix.newHint") });
    for (const prefix of known) {
      const incompatible = prefix.kind !== null && prefix.kind !== runnerKind;
      list.push({
        value: prefix.path,
        label: prefix.name,
        disabled: incompatible,
        hint: incompatible
          ? t("prefix.incompatible", { made: t(`prefix.kind.${prefix.kind}`), runner: t(`prefix.kind.${runnerKind}`) })
          : [t(`prefix.origin.${prefix.origin}`), prefix.kind && t(`prefix.kind.${prefix.kind}`), prefix.arch === "win32" ? "32-bit" : null]
              .filter(Boolean)
              .join(" · "),
      });
    }
    // A prefix chosen by hand, or the one a saved game already has, even if it is not listed.
    if (value !== NEW_PREFIX && !known.some((prefix) => prefix.path === value)) {
      list.push({ value, label: value.split("/").filter(Boolean).slice(-1)[0] ?? value, hint: value });
    }
    list.push({ value: BROWSE, label: t("prefix.browse"), hint: t("prefix.browseHint") });
    return list;
  }, [known, runnerKind, value, allowNew, t]);

  const choose = (next: string) => {
    setError(null);
    if (next !== BROWSE) {
      onChange(next);
      return;
    }
    pickFolder()
      .then((folder) => {
        if (!folder) return;
        return checkPrefix(folder, runnerKind).then(
          () => onChange(folder),
          (reason) => setError(String(reason)),
        );
      })
      .catch((reason) => setError(String(reason)));
  };

  return (
    <div className={styles.picker}>
      <Select ariaLabel={ariaLabel} value={value} options={options} onChange={choose} />
      {error && <p className={styles.error}>{error}</p>}
    </div>
  );
}
