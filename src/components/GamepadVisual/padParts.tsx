import type { CSSProperties, ReactElement } from "react";
import type { ControllerButton, PadSnapshot } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import styles from "./ControllerSvg.module.scss";

/** Helpers shared by the line drawings of the controllers (`ControllerSvg`, `DualSenseSvg`, `SteamDeckSvg`). */
export function padParts(state: PadSnapshot | null, faceColors: Partial<Record<ControllerButton, string>> = {}) {
  const pressed = (button: ControllerButton) => state?.pressed.includes(button) ?? false;

  /** A button's shape, filled while pressed (in its own color for the face buttons). */
  const part = (button: ControllerButton, shape: ReactElement) => (
    <g
      key={button}
      className={cx(styles.part, pressed(button) && styles.on)}
      style={faceColors[button] ? ({ "--pad-on": faceColors[button] } as CSSProperties) : undefined}
    >
      {shape}
    </g>
  );

  /** A stick cap, moved by the analogue values (`travel` is how far it goes, in drawing units). */
  const stick = (button: "l3" | "r3", x: number, y: number, travel: number, caps: ReactElement) => {
    // A stick pushed past the circle (corners of the square range) stays inside the well.
    const length = Math.hypot(x, y);
    const scale = length > 1 ? 1 / length : 1;
    return (
      <g
        className={cx(styles.stick, pressed(button) && styles.on)}
        style={{ transform: `translate(${x * scale * travel}px, ${y * scale * travel}px)` }}
      >
        {caps}
      </g>
    );
  };

  /** A trigger, drawn as an arch above its shoulder button that fills with how far it is pulled. */
  const trigger = (button: "l2" | "r2", value: number, d: string) => (
    <path
      d={d}
      strokeWidth={0.4}
      className={cx(styles.trigger, pressed(button) && styles.on)}
      style={{ "--pull": value } as CSSProperties}
    />
  );

  return { pressed, part, stick, trigger };
}

/** The PlayStation symbol of a face button, drawn small and dim inside it. */
export function PlayStationSymbol({ button, x, y, color, r = 1.05 }: { button: "north" | "west" | "east" | "south"; x: number; y: number; color: string; r?: number }) {
  const common = { stroke: color, strokeWidth: 0.34, fill: "none", strokeLinecap: "round" as const, strokeLinejoin: "round" as const, opacity: 0.85 };
  switch (button) {
    case "north":
      return <path d={`M${x} ${y - r} L${x + r * 0.95} ${y + r * 0.7} H${x - r * 0.95} Z`} {...common} />;
    case "west":
      return <rect x={x - r * 0.85} y={y - r * 0.85} width={r * 1.7} height={r * 1.7} {...common} />;
    case "east":
      return <circle cx={x} cy={y} r={r} {...common} />;
    case "south":
      return <path d={`M${x - r * 0.85} ${y - r * 0.85} L${x + r * 0.85} ${y + r * 0.85} M${x + r * 0.85} ${y - r * 0.85} L${x - r * 0.85} ${y + r * 0.85}`} {...common} />;
  }
}

/** The PlayStation colours of the face buttons. */
export const PLAYSTATION_COLORS = { north: "#4ad8b5", west: "#ff8bd1", east: "#ff6b78", south: "#6aa8ff" } as const;

/** The Xbox layout (Steam Deck): Y on top, X left, B right, A at the bottom. */
export const XBOX_FACE = {
  north: { label: "Y", color: "#ffd24d" },
  west: { label: "X", color: "#5aa9ff" },
  east: { label: "B", color: "#ff6b6b" },
  south: { label: "A", color: "#6dd36d" },
} as const;
