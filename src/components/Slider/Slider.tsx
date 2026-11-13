import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { useInputAction } from "../../input/useInputAction";
import styles from "./Slider.module.scss";

interface SliderProps {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  ariaLabel: string;
  /** How the value is shown next to the slider. */
  format?: (value: number) => string;
}

/** A horizontal slider that works with the mouse, the keyboard and the controller. */
export function Slider({ value, min, max, step, onChange, ariaLabel, format = String }: SliderProps) {
  const track = useRef<HTMLDivElement>(null);
  const ratio = max > min ? (value - min) / (max - min) : 0;

  const commit = (next: number) => {
    const snapped = Math.round((next - min) / step) * step + min;
    const clamped = Math.min(max, Math.max(min, Number(snapped.toFixed(4))));
    if (clamped !== value) onChange(clamped);
  };

  const fromPointer = (event: PointerEvent<HTMLDivElement>) => {
    const rect = track.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return;
    commit(min + ((event.clientX - rect.left) / rect.width) * (max - min));
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const big = (max - min) / 10;
    const next: Record<string, number> = {
      ArrowLeft: value - step,
      ArrowDown: value - step,
      ArrowRight: value + step,
      ArrowUp: value + step,
      PageDown: value - big,
      PageUp: value + big,
      Home: min,
      End: max,
    };
    if (event.key in next) {
      // Keeps the arrows for the slider instead of moving the focus.
      event.preventDefault();
      commit(next[event.key]);
    }
  };

  // With a controller, left/right adjust the slider while it is selected.
  useInputAction((action) => {
    if (document.activeElement !== track.current) return;
    if (action === "left" || action === "right") {
      commit(value + (action === "right" ? step : -step));
      return true;
    }
  });

  return (
    <div className={styles.slider}>
      <div
        ref={track}
        className={styles.track}
        role="slider"
        tabIndex={0}
        data-focusable
        aria-label={ariaLabel}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        aria-valuetext={format(value)}
        onKeyDown={onKeyDown}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          event.currentTarget.focus();
          fromPointer(event);
        }}
        onPointerMove={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId)) fromPointer(event);
        }}
      >
        <span className={styles.fill} style={{ width: `${ratio * 100}%` }} />
        <span className={styles.thumb} style={{ left: `${ratio * 100}%` }} />
      </div>
      <span className={styles.value}>{format(value)}</span>
    </div>
  );
}
