import { memo, type CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { SystemIcon } from "../../components/SystemIcon/SystemIcon";
import { hoverFocus } from "../../input/hoverFocus";
import type { SystemEntry } from "../../hooks/useEmulation";
import styles from "./Emulation.module.scss";

/** A console on the dashboard: its picture, its name, how many games, whether it can play. */
function SystemTileView({
  system,
  autofocus = false,
  onOpen,
}: {
  system: SystemEntry;
  autofocus?: boolean;
  onOpen: (name: string) => void;
}) {
  const { t } = useTranslation();
  const { platform } = system;

  return (
    <button
      type="button"
      data-focusable
      data-autofocus={autofocus ? "" : undefined}
      className={styles.tile}
      style={{ "--accent": platform.color } as CSSProperties}
      onClick={() => onOpen(system.name)}
      {...hoverFocus}
    >
      <span className={styles.tileGlow} aria-hidden="true" />
      <SystemIcon platform={platform} icon={system.icon} className={styles.tileIcon} />
      <span className={styles.tileText}>
        <strong>{platform.name}</strong>
        <small>{[platform.maker, platform.year].filter(Boolean).join(" · ")}</small>
      </span>
      <span className={styles.tileFooter}>
        <span className={styles.count}>{t("emulation.games", { count: system.gameCount })}</span>
        <span className={styles.state} data-state={system.state}>
          {t(`emulation.state.${system.state}`)}
        </span>
      </span>
    </button>
  );
}

export const SystemTile = memo(SystemTileView);
