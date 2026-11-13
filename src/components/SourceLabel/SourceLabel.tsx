import { useTranslation } from "react-i18next";
import { cx } from "../../lib/cx";
import { SOURCE_COLOR } from "../../lib/sources";
import type { GameSource } from "../../types/game";
import styles from "./SourceLabel.module.scss";

/** A source's name preceded by its color dot. */
export function SourceLabel({ source, className }: { source: GameSource; className?: string }) {
  const { t } = useTranslation();
  return (
    <span className={cx(styles.label, className)}>
      <i style={{ background: SOURCE_COLOR[source] }} />
      {t(`sources.${source}`)}
    </span>
  );
}
