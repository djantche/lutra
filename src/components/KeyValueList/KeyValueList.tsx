import { useTranslation } from "react-i18next";
import type { EnvVar } from "../../lib/backend";
import { Button } from "../Button/Button";
import { Icon } from "../Icon/Icon";
import { TextField } from "../TextField/TextField";
import styles from "./KeyValueList.module.scss";

interface KeyValueListProps {
  items: EnvVar[];
  onChange: (items: EnvVar[]) => void;
  keyPlaceholder: string;
  valuePlaceholder: string;
  addLabel: string;
}

/** Editable list of name/value pairs, e.g. environment variables. */
export function KeyValueList({ items, onChange, keyPlaceholder, valuePlaceholder, addLabel }: KeyValueListProps) {
  const { t } = useTranslation();
  const update = (index: number, patch: Partial<EnvVar>) =>
    onChange(items.map((item, position) => (position === index ? { ...item, ...patch } : item)));

  return (
    <div className={styles.list}>
      {items.map((item, index) => (
        <div key={index} className={styles.row}>
          <TextField
            className={styles.key}
            value={item.key}
            placeholder={keyPlaceholder}
            aria-label={keyPlaceholder}
            spellCheck={false}
            onChange={(key) => update(index, { key })}
          />
          <TextField
            className={styles.value}
            value={item.value}
            placeholder={valuePlaceholder}
            aria-label={valuePlaceholder}
            spellCheck={false}
            onChange={(value) => update(index, { value })}
          />
          <button
            type="button"
            className={styles.remove}
            aria-label={t("common.remove")}
            onClick={() => onChange(items.filter((_, position) => position !== index))}
          >
            <Icon name="close" size={14} />
          </button>
        </div>
      ))}
      <Button variant="ghost" icon="plus" onClick={() => onChange([...items, { key: "", value: "" }])}>
        {addLabel}
      </Button>
    </div>
  );
}
