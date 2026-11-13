import styles from "./LaunchRing.module.scss";

interface LaunchRingProps {
  /** Stages finished so far. */
  step: number;
  steps: number;
  /** Diameter in px; the ring follows the text size when absent. */
  size?: number;
}

const RADIUS = 15;
const GAP = 3;

/**
 * Circular launch progress made of one arc per real stage: finished stages are filled, the stage
 * in progress pulses (its duration is not known), the others wait. Nothing here is a guess.
 */
export function LaunchRing({ step, steps, size }: LaunchRingProps) {
  const total = Math.max(1, steps);
  const span = 100 / total;
  const length = Math.max(span - GAP, 1);

  return (
    <svg
      className={styles.ring}
      viewBox="0 0 36 36"
      style={size ? { width: size, height: size } : undefined}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={total}
      aria-valuenow={Math.min(step, total)}
    >
      {Array.from({ length: total }, (_, index) => {
        const state = index < step ? styles.done : index === step ? styles.current : styles.todo;
        return (
          <circle
            key={index}
            className={`${styles.arc} ${state}`}
            cx="18"
            cy="18"
            r={RADIUS}
            pathLength={100}
            strokeDasharray={`${length} ${100 - length}`}
            strokeDashoffset={-(index * span + GAP / 2)}
          />
        );
      })}
    </svg>
  );
}
