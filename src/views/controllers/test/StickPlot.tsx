import styles from "./ControllerTestScreen.module.scss";

const SIZE = 120;
const RADIUS = 52;

interface StickPlotProps {
  label: string;
  raw: [number, number];
  output: [number, number];
  deadzone: number;
}

/** A stick seen from above: where it physically is (grey), what the game gets (colour), and the dead zone. */
export function StickPlot({ label, raw, output, deadzone }: StickPlotProps) {
  const at = ([x, y]: [number, number]) => ({ cx: SIZE / 2 + x * RADIUS, cy: SIZE / 2 + y * RADIUS });
  const centre = SIZE / 2;

  return (
    <figure className={styles.meter}>
      <svg viewBox={`0 0 ${SIZE} ${SIZE}`} className={styles.plot} aria-hidden="true">
        <circle cx={centre} cy={centre} r={RADIUS} className={styles.ring} />
        <circle cx={centre} cy={centre} r={Math.max(0.5, deadzone * RADIUS)} className={styles.deadzone} />
        <line x1={centre - RADIUS} y1={centre} x2={centre + RADIUS} y2={centre} className={styles.axis} />
        <line x1={centre} y1={centre - RADIUS} x2={centre} y2={centre + RADIUS} className={styles.axis} />
        <circle {...at(raw)} r={5} className={styles.rawDot} />
        <circle {...at(output)} r={6.5} className={styles.outDot} />
      </svg>
      <figcaption>
        {label}
        <small>
          {output[0].toFixed(2)} · {output[1].toFixed(2)}
        </small>
      </figcaption>
    </figure>
  );
}
