import { useTranslation } from "react-i18next";
import type { ControllerButton, PadSnapshot } from "../../lib/controllers";
import { padParts, XBOX_FACE } from "./padParts";
import styles from "./ControllerSvg.module.scss";

// A Steam Deck drawn as line art (from `controller-3.svg`): see `ControllerSvg` for how it is used.
// The triggers (not part of the drawing) are arches above the shoulders.

const STICK_TRAVEL = 1.0;
const FACE_POSITIONS = { north: [71.6, 29.81], west: [69.17, 32.23], east: [74.03, 32.23], south: [71.62, 34.75] } as const;
const FACE_COLORS: Partial<Record<ControllerButton, string>> = {
  north: XBOX_FACE.north.color,
  west: XBOX_FACE.west.color,
  east: XBOX_FACE.east.color,
  south: XBOX_FACE.south.color,
};

/** The D-pad is one shape in the drawing: each direction is an arm drawn over it. */
const DPAD_ARMS = {
  dpadUp: [7.5, 29.1, 2.2, 3.2],
  dpadDown: [7.5, 32.3, 2.2, 3.2],
  dpadLeft: [5.4, 31.2, 3.2, 2.2],
  dpadRight: [8.6, 31.2, 3.2, 2.2],
} as const;

/** The Steam Deck, live when given a state: pressed buttons fill, sticks move, triggers fill with their pull. */
export function SteamDeckSvg({ state }: { state: PadSnapshot | null }) {
  const { t } = useTranslation();
  const { part, stick, trigger } = padParts(state, FACE_COLORS);

  return (
    <svg className={styles.pad} viewBox="2 19.5 76 37" role="img" aria-label={t("controllers.liveView")}>
      <g className={styles.line}>
        <path d="m76.74 33.26c-0.09-2.5-1.23-4.47-2.72-5.69-0.26-0.86-0.77-1.76-1.84-2.38-1.43-0.83-2.97-1.04-4.33-1.04s-3.11 0.2-3.7 0.33-0.78 0.73-0.93 1.1h-46.54c-0.15-0.37-0.43-0.97-1.02-1.1-0.6-0.13-2.35-0.33-3.61-0.33-1.36 0-2.8 0.21-4.23 1.04-1.17 0.62-1.68 1.67-1.91 2.38-1.49 1.27-2.52 3.32-2.62 5.69-0.15 3.19-0.16 6.13-0.14 9.69 0 3.09 0.5 5.75 1.22 7.74 1.18 3.24 3.98 4.8 7.72 4.8 4.39 0 12.07-0.08 27.91-0.08s23.52 0.08 27.91 0.08c3.74 0 6.56-1.56 7.72-4.8 0.77-2.2 1.22-4.85 1.22-7.74 0.02-3.56 0.01-6.72-0.11-9.69z" strokeWidth={.5345} />
        <path d="m5.91 27.57c1.72-1.37 4.49-1.99 7.34-1.99h53.5c2.85 0 5.72 0.67 7.27 1.99" strokeWidth={.5345} />
        <path d="m3.34 36.19c2.02 2.12 3.1 5.1 3.84 8.13 0.53 2.1 1.03 4.2 1.91 5.86 1.56 2.94 4.29 5.07 7.12 5.16" strokeWidth={.281} />
        <path d="m76.65 36.19c-2.09 2.12-3.13 5.32-3.88 8.35-0.52 2.07-1.03 4.11-1.91 5.74-1.64 3.01-4.41 4.94-7.07 5.06" strokeWidth={.281} />
        <rect x={20.69} y={27.24} width={38.64} height={26.46} rx={1.015} strokeWidth={.3121} />
        <path d="m22.16 28.84c0-0.16 0.12-0.28 0.28-0.28h35.15c0.15 0 0.22 0.1 0.22 0.25v23.15c0 0.16-0.1 0.26-0.27 0.26l-8.67-0.09-1.96 0.05-24.47 0.04c-0.15 0-0.28-0.1-0.28-0.27v-23.11z" strokeWidth={.4234} />
        <circle cx={16.86} cy={32.12} r={3.419} strokeWidth={.4086} />
        <circle cx={63.08} cy={32.12} r={3.419} strokeWidth={.4086} />
        <rect x={9.965} y={36.92} width={9.404} height={8.587} rx={1.328} strokeWidth={.3928} />
        <rect x={60.64} y={36.92} width={9.314} height={8.587} rx={1.284} strokeWidth={.3928} />
        <path d="m7.49 29.4v1.52c0 0.12-0.1 0.19-0.21 0.19h-1.51c-0.29 0-0.48 0.22-0.48 0.48v1.32c0 0.3 0.21 0.52 0.48 0.52h1.49c0.15 0 0.23 0.09 0.23 0.21v1.44c0 0.28 0.23 0.49 0.49 0.49h1.24c0.28 0 0.49-0.2 0.49-0.48v-1.44c0-0.13 0.07-0.22 0.2-0.22h1.5c0.28 0 0.48-0.21 0.48-0.51v-1.31c0-0.29-0.2-0.5-0.49-0.5h-1.49c-0.14 0-0.2-0.08-0.2-0.19v-1.5c0-0.25-0.21-0.41-0.46-0.41h-1.31c-0.27 0-0.45 0.17-0.45 0.39z" strokeWidth={.3813} />
      </g>
      <g className={styles.dots}>
        <circle cx={14.67} cy={50.73} r={.3286} />
        <circle cx={16.06} cy={50.73} r={.3286} />
        <circle cx={17.4} cy={50.73} r={.3286} />
        <circle cx={18.76} cy={50.73} r={.3286} />
        <circle cx={15.38} cy={51.43} r={.3121} />
        <circle cx={16.73} cy={51.43} r={.3121} />
        <circle cx={18.12} cy={51.43} r={.3121} />
        <circle cx={14.69} cy={52.12} r={.3208} />
        <circle cx={16.07} cy={52.12} r={.3208} />
        <circle cx={17.41} cy={52.12} r={.3208} />
        <circle cx={18.74} cy={52.12} r={.3208} />
        <circle cx={61.24} cy={50.73} r={.3286} />
        <circle cx={62.48} cy={50.73} r={.3286} />
        <circle cx={63.92} cy={50.73} r={.3286} />
        <circle cx={65.28} cy={50.73} r={.3286} />
        <circle cx={61.9} cy={51.43} r={.3121} />
        <circle cx={63.23} cy={51.43} r={.3121} />
        <circle cx={64.62} cy={51.43} r={.3121} />
        <circle cx={61.25} cy={52.12} r={.3208} />
        <circle cx={62.51} cy={52.12} r={.3208} />
        <circle cx={63.91} cy={52.12} r={.3208} />
        <circle cx={65.27} cy={52.12} r={.3208} />
      </g>

      <g className={styles.line}>
        {trigger("l2", state?.lt ?? 0, "M7 23.8 C7.6 21.4 9.6 20.6 11.6 20.6 C13.6 20.6 15.6 21.4 16.2 23.8 Z")}
        {trigger("r2", state?.rt ?? 0, "M63.8 23.8 C64.4 21.4 66.4 20.6 68.4 20.6 C70.4 20.6 72.4 21.4 73 23.8 Z")}
        {part("l1", <rect x={11.96} y={27.05} width={3.167} height={1.272} rx={.5835} strokeWidth={.2945} />)}
        {part("r1", <rect x={64.95} y={27.05} width={3.19} height={1.272} rx={.5835} strokeWidth={.2945} />)}
        {part("select", <rect x={14.91} y={46.77} width={4.319} height={1.727} rx={.8175} strokeWidth={.3052} />)}
        {part("start", <rect x={60.96} y={46.77} width={4.203} height={1.727} rx={.771} strokeWidth={.3052} />)}
        {(Object.keys(DPAD_ARMS) as (keyof typeof DPAD_ARMS)[]).map((button) => {
          const [x, y, width, height] = DPAD_ARMS[button];
          return part(button, <rect x={x} y={y} width={width} height={height} rx={0.45} stroke="none" />);
        })}
        {part("north", <circle cx={71.6} cy={29.81} r={1.224} strokeWidth={.3638} />)}
        {part("west", <circle cx={69.17} cy={32.23} r={1.224} strokeWidth={.3638} />)}
        {part("east", <circle cx={74} cy={32.23} r={1.224} strokeWidth={.3638} />)}
        {part("south", <circle cx={71.6} cy={34.71} r={1.245} strokeWidth={.3638} />)}
      </g>

      {(Object.keys(FACE_POSITIONS) as (keyof typeof FACE_POSITIONS)[]).map((button) => (
        <text
          key={button}
          x={FACE_POSITIONS[button][0]}
          y={FACE_POSITIONS[button][1] + 0.55}
          fill={XBOX_FACE[button].color}
          fontSize={1.7}
          fontWeight={700}
          textAnchor="middle"
          opacity={0.85}
        >
          {XBOX_FACE[button].label}
        </text>
      ))}

      <g className={styles.line}>
        {stick("l3", state?.lx ?? 0, state?.ly ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={16.87} cy={32.04} r={2.437} strokeWidth={.4086} />
          </>
        ))}
        {stick("r3", state?.rx ?? 0, state?.ry ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={63.09} cy={32.04} r={2.437} strokeWidth={.4086} />
          </>
        ))}
      </g>
    </svg>
  );
}
