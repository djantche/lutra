import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BatteryGauge } from "../../components/BatteryGauge/BatteryGauge";
import { Icon } from "../../components/Icon/Icon";
import { useControllerBatteries } from "../../views/controllers/useControllers";
import styles from "./StatusCluster.module.scss";

function useClock(language: string): string {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(id);
  }, []);
  return new Intl.DateTimeFormat(language, { hour: "2-digit", minute: "2-digit" }).format(now);
}

/**
 * The corner of the screen that says how the machine is doing: the charge of each controller, with
 * which controller it is (its kind, numbered when several of a kind are connected), and the time.
 */
export function StatusCluster() {
  const { t, i18n } = useTranslation();
  const clock = useClock(i18n.language);
  const batteries = useControllerBatteries();

  const seen = new Map<string, number>();
  const total = (family: string) => batteries.filter((entry) => entry.family === family).length;

  return (
    <div className={styles.cluster}>
      {batteries.map(({ id, name, family, battery }) => {
        const number = (seen.get(family) ?? 0) + 1;
        seen.set(family, number);
        const kind = t(`controllers.family.${family}`);
        const label = total(family) > 1 ? `${kind} ${number}` : kind;
        return (
          <span key={id} className={styles.pad} title={name}>
            <Icon name="gamepad" size={22} />
            {/* With a single controller its icon is enough; with several, which is which matters. */}
            {batteries.length > 1 && <span className={styles.name}>{label}</span>}
            <BatteryGauge battery={battery} light />
          </span>
        );
      })}
      <time className={styles.clock}>{clock}</time>
    </div>
  );
}
