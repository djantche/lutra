import styles from "./ControllerTestScreen.module.scss";

interface TriggerMeterProps {
  label: string;
  raw: number;
  output: number;
}

/** A trigger's travel: pressed (grey) and what the game gets after the dead zone (colour). */
export function TriggerMeter({ label, raw, output }: TriggerMeterProps) {
  return (
    <figure className={styles.meter}>
      <div className={styles.trigger} aria-hidden="true">
        <span className={styles.triggerRaw} style={{ height: `${raw * 100}%` }} />
        <span className={styles.triggerOut} style={{ height: `${output * 100}%` }} />
      </div>
      <figcaption>
        {label}
        <small>{Math.round(output * 100)} %</small>
      </figcaption>
    </figure>
  );
}
