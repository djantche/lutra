import { useTranslation } from "react-i18next";
import { cx } from "../../lib/cx";
import { SourceIcon } from "../SourceIcon/SourceIcon";
import type { GameSource } from "../../types/game";
import styles from "./SourceLabel.module.scss";

/** A source's name preceded by its logo. */
export function SourceLabel({ source, className }: { source: GameSource; className?: string }) {
  const { t } = useTranslation();
  return (
    <span className={cx(styles.label, className)}>
      <SourceIcon source={source} size={14} />
      {t(`sources.${source}`)}
    </span>
  );
}
