import { useTranslation } from "react-i18next";
import { BatteryGauge } from "../../components/BatteryGauge/BatteryGauge";
import { Icon } from "../../components/Icon/Icon";
import type { ControllerInfo } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import styles from "./Controllers.module.scss";

interface ControllerListProps {
  controllers: ControllerInfo[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

/** Connected controllers, one card each. */
export function ControllerList({ controllers, selectedId, onSelect }: ControllerListProps) {
  const { t } = useTranslation();

  return (
    <div className={styles.list}>
      {controllers.map((controller) => (
        <button
          key={controller.id}
          type="button"
          data-focusable
          className={cx(styles.device, controller.id === selectedId && styles.selected)}
          aria-pressed={controller.id === selectedId}
          onClick={() => onSelect(controller.id)}
        >
          <span className={styles.deviceIcon}>
            <Icon name="gamepad" size={22} />
          </span>
          <span className={styles.deviceText}>
            <strong>{t(`controllers.family.${controller.family}`)}</strong>
            <small>
              {controller.name} · {t(`controllers.bus.${controller.bus}`)}
            </small>
          </span>
          {controller.battery && <BatteryGauge battery={controller.battery} />}
        </button>
      ))}
    </div>
  );
}
