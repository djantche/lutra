import type { CSSProperties } from "react";
import { cx } from "../../lib/cx";
import type { Platform } from "../../lib/platforms";
import { useArtworkUrl } from "../../lib/useArtworkUrl";
import styles from "./SystemIcon.module.scss";

/**
 * The picture of a console (from RetroArch's assets), or its short name on its colour when
 * none is installed. Size it through the parent's font-size.
 */
export function SystemIcon({ platform, icon, className }: { platform: Platform; icon: string | null; className?: string }) {
  const url = useArtworkUrl(icon ?? undefined, undefined);
  return (
    <span className={cx(styles.icon, className)} style={{ "--accent": platform.color } as CSSProperties}>
      {url ? (
        <img src={url} alt="" draggable={false} />
      ) : (
        <span className={styles.badge}>{platform.short}</span>
      )}
    </span>
  );
}
