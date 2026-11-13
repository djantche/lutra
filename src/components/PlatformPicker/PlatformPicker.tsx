import { useMemo, useState, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import type { SystemEntry } from "../../hooks/useEmulation";
import { makerRank } from "../../lib/platforms";
import { cx } from "../../lib/cx";
import { SystemIcon } from "../SystemIcon/SystemIcon";
import { TextField } from "../TextField/TextField";
import styles from "./PlatformPicker.module.scss";

interface PlatformPickerProps {
  systems: SystemEntry[];
  value: string;
  onChange: (system: string) => void;
}

/** Picks a console among those Lutra knows, with a search for the long list. */
export function PlatformPicker({ systems, value, onChange }: PlatformPickerProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");

  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return systems
      .filter((system) => system.platform.main || system.cores.length > 0 || system.name === value)
      .filter((system) => {
        if (!needle) return true;
        const { short, name, maker } = system.platform;
        return `${short} ${name} ${maker} ${system.name}`.toLowerCase().includes(needle);
      })
      .sort(
        (a, b) =>
          makerRank(a.platform.maker) - makerRank(b.platform.maker) ||
          a.platform.maker.localeCompare(b.platform.maker) ||
          (a.platform.year ?? 9999) - (b.platform.year ?? 9999),
      );
  }, [systems, query, value]);

  return (
    <div className={styles.picker}>
      <TextField icon="search" clearable value={query} placeholder={t("addGame.searchConsole")} onChange={setQuery} />
      <div className={styles.grid} role="radiogroup" aria-label={t("addGame.console")}>
        {shown.map((system) => (
          <button
            key={system.name}
            type="button"
            role="radio"
            aria-checked={system.name === value}
            data-focusable
            className={cx(styles.option, system.name === value && styles.selected)}
            style={{ "--accent": system.platform.color } as CSSProperties}
            onClick={() => onChange(system.name)}
          >
            <SystemIcon platform={system.platform} icon={system.icon} className={styles.icon} />
            <span className={styles.text}>
              <strong>{system.platform.name}</strong>
              <small>
                {system.state === "noCore" ? t("emulation.state.noCore") : system.platform.maker || t("emulation.otherMakers")}
              </small>
            </span>
          </button>
        ))}
        {shown.length === 0 && <p className={styles.none}>{t("addGame.noConsole")}</p>}
      </div>
    </div>
  );
}
