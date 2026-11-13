import type { PadSnapshot } from "../../lib/controllers";
import { useTranslation } from "react-i18next";
import { padParts, PlayStationSymbol, PLAYSTATION_COLORS } from "./padParts";
import styles from "./ControllerSvg.module.scss";

// A DualShock 4 drawn as line art (80 × 80 units, from `controller.svg`). Each part below is one of
// its shapes: a pressed button is filled, the sticks move with the analogue values, and the
// triggers (not part of the drawing) are two arches above the shoulders, filled by their pull.

/** How far a stick cap travels from its centre, in drawing units (the well is about 7 wide). */
const STICK_TRAVEL = 2.3;

/** Where each face button is, for its symbol. */
const FACE = {
  north: { x: 63.89, y: 24.57 },
  west: { x: 58.71, y: 29.85 },
  east: { x: 69.27, y: 29.9 },
  south: { x: 63.99, y: 35.26 },
} as const;

/** The DualShock 4, live when given a state: pressed buttons fill, sticks move, triggers fill with their pull. */
export function ControllerSvg({ state }: { state: PadSnapshot | null }) {
  const { t } = useTranslation();
  const { part, stick: moveStick, trigger } = padParts(state, PLAYSTATION_COLORS);
  const stick = (button: "l3" | "r3", x: number, y: number, caps: React.ReactElement) =>
    moveStick(button, x, y, STICK_TRAVEL, caps);

  return (
    <svg className={styles.pad} viewBox="0 9 80 60" role="img" aria-label={t("controllers.liveView")}>
      <g className={styles.line}>
        <path d="m23.91 17.75h3.65" strokeWidth={.5644} />
        <path d="m52.44 17.75h3.65" strokeWidth={.5644} />
        <path d="m33.11 47.26h13.72" strokeWidth={.5644} />
        <path d="m22.86 47.4c-1.34-0.03-2.08 0.71-2.61 2.19-1.12 3.21-2.47 6.9-3.95 10.26-1.48 3.48-4.09 5.32-7.15 5.32-4.25 0-7.77-3.23-8.14-10.08-0.47-6.88 1.25-15.17 4.36-25.91 1.1-4.11 3.15-8.24 5.28-11.32 0.2-0.29 0.56-0.54 1.29-0.73 3.32-0.87 7.6-0.85 10.62-0.39 0.66 0.12 1.09 0.45 1.35 0.95" strokeWidth={.6345} />
        <path d="m57.03 47.4c1.34-0.03 2.07 0.78 2.59 2.19 1.17 3.32 2.61 7.1 4.13 10.37 1.54 3.56 4.18 5.21 7.1 5.21 3.99 0 7.6-3.05 8.05-9.91 0.64-6.92-1.21-15.51-4.38-26.3-1.03-4.1-3.27-8.35-5.2-11.13-0.21-0.31-0.65-0.57-1.26-0.72-3.41-0.85-7.45-0.82-10.5-0.37-0.68 0.12-1.19 0.44-1.47 0.95" strokeWidth={.6345} />
        <path d="m1.15 54.91c3 4.66 10.56 6.57 15.92 2.97" strokeWidth={.423} />
        <path d="m62.84 57.83c5.41 3.86 13.41 1.82 16.01-2.92" strokeWidth={.423} />
        <path d="m22.89 47.41c1.37 1.24 3.2 1.88 5.09 1.88s3.84-0.87 5.06-2.06" strokeWidth={.423} />
        <path d="m46.86 47.25c1.43 1.39 3.36 2.06 5.17 2.06 1.8 0 3.74-0.79 4.95-1.95" strokeWidth={.423} />
        <path d="m51.45 17.52h-22.9c-0.8 0-1.29 0.51-1.29 1.3v11.62c0 0.98 0.83 1.69 1.79 1.69h22.04c0.99 0 1.73-0.65 1.73-1.61v-11.7c0-0.81-0.54-1.3-1.37-1.3z" strokeWidth={.4831} />
        <circle cx={28.03} cy={41.68} r={7.254} strokeWidth={.4831} />
        <circle cx={51.9} cy={41.68} r={7.344} strokeWidth={.4831} />
        <rect x={6.886} y={20.55} width={18.2} height={18.92} rx={9.101} strokeWidth={.442} />
        <rect x={54.85} y={20.55} width={18.24} height={18.94} rx={9.119} strokeWidth={.423} />
      </g>
      <g className={styles.dots}>
        <circle cx={36.8} cy={34.36} r={.3342} />
        <circle cx={38.15} cy={34.36} r={.3342} />
        <circle cx={39.36} cy={34.36} r={.3342} />
        <circle cx={40.67} cy={34.36} r={.3342} />
        <circle cx={41.95} cy={34.36} r={.3342} />
        <circle cx={43.23} cy={34.36} r={.3342} />
        <circle cx={37.42} cy={35.49} r={.3342} />
        <circle cx={38.76} cy={35.49} r={.3342} />
        <circle cx={39.98} cy={35.49} r={.3342} />
        <circle cx={41.27} cy={35.49} r={.3342} />
        <circle cx={42.6} cy={35.49} r={.3342} />
        <circle cx={38.15} cy={36.7} r={.3342} />
        <circle cx={39.36} cy={36.7} r={.3342} />
        <circle cx={40.67} cy={36.7} r={.3342} />
        <circle cx={41.95} cy={36.7} r={.3342} />
      </g>

      <g className={styles.line}>
        {trigger("l2", state?.lt ?? 0, "M12.7 14 C13.3 11.4 15.5 10.6 17.9 10.6 C20.4 10.6 22.5 11.4 23.1 14 Z")}
        {trigger("r2", state?.rt ?? 0, "M58.3 14 C58.9 11.4 61 10.6 63.4 10.6 C65.8 10.6 67.9 11.4 68.5 14 Z")}
        {part("l1", <path d="m11.96 17.07c-0.12-0.74 0.15-1.28 0.73-1.48 2.85-1.14 6.5-1.06 8.95-0.14 0.57 0.2 0.76 0.67 0.71 1.21" strokeWidth={.4831} />)}
        {part("r1", <path d="m57.66 16.65c-0.08-0.6 0.17-1.08 0.73-1.25 2.83-1.02 6.31-0.79 8.86 0.18 0.72 0.26 0.96 0.78 0.85 1.48" strokeWidth={.4831} />)}
        {part("select", <rect x={23.39} y={19.64} width={2.301} height={4.27} rx={.9508} strokeWidth={.423} />)}
        {part("start", <rect x={54.34} y={19.64} width={2.242} height={4.27} rx={.9508} strokeWidth={.423} />)}
        {part("guide", <circle cx={39.98} cy={41.64} r={2.168} strokeWidth={.423} />)}
        {part("dpadUp", <path d="m15.29 23.03h1.75c0.71 0 1.22 0.47 1.22 1.19v2.3c0 0.26-0.09 0.47-0.28 0.65l-1.3 1.26c-0.36 0.32-0.91 0.32-1.25 0l-1.37-1.29c-0.18-0.17-0.24-0.37-0.24-0.62v-2.26c0-0.76 0.62-1.23 1.47-1.23z" strokeWidth={.4662} />)}
        {part("dpadDown", <path d="m15.29 36.93h1.75c0.76 0 1.27-0.49 1.27-1.19v-2.32c0-0.25-0.08-0.46-0.27-0.63l-1.34-1.28c-0.35-0.33-0.91-0.33-1.24 0l-1.37 1.3c-0.18 0.17-0.24 0.36-0.24 0.61v2.27c0 0.73 0.62 1.24 1.44 1.24z" strokeWidth={.4662} />)}
        {part("dpadRight", <path d="m22.91 29.08v1.84c0 0.73-0.53 1.24-1.23 1.24h-2.22c-0.24 0-0.45-0.07-0.63-0.24l-1.34-1.34c-0.34-0.33-0.32-0.89 0-1.22l1.31-1.33c0.18-0.2 0.4-0.29 0.66-0.29h2.19c0.77 0 1.26 0.53 1.26 1.34z" strokeWidth={.4662} />)}
        {part("dpadLeft", <path d="m9.14 29.08v1.76c0 0.78 0.53 1.32 1.21 1.32h2.24c0.25 0 0.47-0.08 0.65-0.25l1.32-1.33c0.34-0.34 0.32-0.9 0-1.22l-1.29-1.32c-0.18-0.19-0.4-0.28-0.66-0.28h-2.23c-0.72 0-1.24 0.55-1.24 1.32z" strokeWidth={.4662} />)}
        {part("north", <circle cx={63.89} cy={24.57} r={2.498} strokeWidth={.4662} />)}
        {part("west", <circle cx={58.71} cy={29.85} r={2.475} strokeWidth={.4662} />)}
        {part("east", <circle cx={69.27} cy={29.9} r={2.463} strokeWidth={.4662} />)}
        {part("south", <circle cx={63.99} cy={35.26} r={2.552} strokeWidth={.4662} />)}
      </g>

      {(Object.keys(FACE) as (keyof typeof FACE)[]).map((button) => (
        <PlayStationSymbol key={button} button={button} x={FACE[button].x} y={FACE[button].y} color={PLAYSTATION_COLORS[button]} />
      ))}

      <g className={styles.line}>
        {stick("l3", state?.lx ?? 0, state?.ly ?? 0, (
          <>
          <circle cx={27.95} cy={41.57} r={4.729} strokeWidth={.442} />
          <circle cx={27.92} cy={41.55} r={3.395} strokeWidth={.442} />
          </>
        ))}
        {stick("r3", state?.rx ?? 0, state?.ry ?? 0, (
          <>
          <circle cx={51.96} cy={41.57} r={4.694} strokeWidth={.442} />
          <circle cx={51.94} cy={41.57} r={3.374} strokeWidth={.442} />
          </>
        ))}
      </g>
    </svg>
  );
}
