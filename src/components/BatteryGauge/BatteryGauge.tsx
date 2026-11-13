import { useTranslation } from "react-i18next";
import type { ControllerBattery } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import styles from "./BatteryGauge.module.scss";

/** A controller's charge: a small battery that fills, the percentage, and a bolt while charging. */
export function BatteryGauge({ battery }: { battery: ControllerBattery }) {
  const { t } = useTranslation();
  const level = battery.percent <= 15 ? "critical" : battery.percent <= 30 ? "low" : "ok";
  const label = `${battery.charging ? t("controllers.battery.charging") : t("controllers.battery.label")}: ${battery.percent} %`;

  return (
    <span
      className={cx(styles.gauge, styles[level])}
      role="img"
      aria-label={label}
      title={label}
    >
      <svg viewBox="0 0 28 14" width="2em" height="1em" aria-hidden="true">
        <rect x="0.75" y="0.75" width="23.5" height="12.5" rx="3" fill="none" stroke="currentColor" strokeWidth="1.5" opacity="0.7" />
        <rect x="25" y="4.5" width="2.5" height="5" rx="1" fill="currentColor" opacity="0.7" />
        <rect x="2.5" y="2.5" width={Math.max(1, (battery.percent / 100) * 20)} height="9" rx="1.6" className={styles.fill} />
        {battery.charging && <path d="M13.5 2.5 8.5 8h3l-1 3.5 5-5.5h-3z" fill="#fff" stroke="#000" strokeWidth="0.4" strokeLinejoin="round" />}
      </svg>
      <span>{battery.percent} %</span>
    </span>
  );
}
