import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { buttonLabel } from "../../../lib/buttonLabels";
import type { ControllerFamily } from "../../../lib/controllers";
import styles from "./ControllerTestScreen.module.scss";

const HOLD_MS = 1200;
const RING = 2 * Math.PI * 22;

interface HoldToExitProps {
  /** Whether the exit button is held right now. */
  holding: boolean;
  family: ControllerFamily;
  onExit: () => void;
}

/**
 * Leaving the test needs a long press, so testing that button does not close the screen.
 * The ring fills while the button is held and empties as soon as it is released.
 */
export function HoldToExit({ holding, family, onExit }: HoldToExitProps) {
  const { t } = useTranslation();
  const [progress, setProgress] = useState(0);
  const since = useRef<number | null>(null);

  useEffect(() => {
    if (!holding) {
      since.current = null;
      setProgress(0);
      return;
    }
    since.current ??= performance.now();
    let frame = 0;
    const step = () => {
      const elapsed = performance.now() - (since.current ?? performance.now());
      const next = Math.min(1, elapsed / HOLD_MS);
      setProgress(next);
      if (next >= 1) onExit();
      else frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [holding, onExit]);

  return (
    <div className={styles.exit} data-holding={holding}>
      <svg viewBox="0 0 52 52" className={styles.exitRing} aria-hidden="true">
        <circle cx={26} cy={26} r={22} className={styles.exitTrack} />
        <circle
          cx={26}
          cy={26}
          r={22}
          className={styles.exitFill}
          strokeDasharray={RING}
          strokeDashoffset={RING * (1 - progress)}
        />
        <text x={26} y={31} className={styles.exitGlyph}>
          {buttonLabel("east", family, t)}
        </text>
      </svg>
      <span>{t("controllers.test.holdToExit")}</span>
    </div>
  );
}
