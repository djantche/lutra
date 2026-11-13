import { useTranslation } from "react-i18next";
import type { PadSnapshot } from "../../lib/controllers";
import { padParts, PlayStationSymbol, PLAYSTATION_COLORS } from "./padParts";
import styles from "./ControllerSvg.module.scss";

// A DualSense drawn as line art (from `controller-2.svg`): see `ControllerSvg` for how it is used.
// The sticks are measured on the drawing; the triggers (not part of it) are arches above the bumpers.

const STICK_TRAVEL = 1.3;
const FACE_POSITIONS = { north: [58.52, 28.86], west: [54.4, 32.8], east: [62.6, 32.8], south: [58.52, 36.81] } as const;

/** The DualSense, live when given a state: pressed buttons fill, sticks move, triggers fill with their pull. */
export function DualSenseSvg({ state }: { state: PadSnapshot | null }) {
  const { t } = useTranslation();
  const { part, stick, trigger } = padParts(state, PLAYSTATION_COLORS);

  return (
    <svg className={styles.pad} viewBox="8 16.5 64 44" role="img" aria-label={t("controllers.liveView")}>
      <g className={styles.line}>
        <path d="m62.61 25.19c-2.42-1.06-6.9-1.8-9.15-1.98-0.66-0.05-1.24 0.4-1.37 1.05l-1.58 7.75c-0.25 1.37 0.17 2.6 1.23 3.66 3.68 3.64 7.42 8.25 11.87 21.81 0.21 0.57 0.54 0.9 0.85 0.88 3.15-1.14 4.81-4.68 4.81-10.31 0-9.83-4.58-21.62-6.66-22.86z" strokeWidth={.4444} />
        <path d="m17.39 25.19c2.42-1.06 6.9-1.8 9.15-1.98 0.66-0.05 1.24 0.4 1.37 1.05l1.58 7.75c0.25 1.37-0.17 2.6-1.23 3.66-3.68 3.64-7.42 8.25-11.87 21.81-0.21 0.57-0.54 0.9-0.85 0.88-3.15-1.14-4.82-4.68-4.82-10.31 0-9.83 4.59-21.62 6.67-22.86z" strokeWidth={.4444} />
        <path d="m27.61 23.78 1.26-0.12" strokeWidth={.3333} />
        <path d="m52.22 23.78-1.23-0.14" strokeWidth={.3333} />
        <path d="m49.77 23.02c-3.14-0.25-6.55-0.32-9.77-0.32s-6.63 0.07-9.87 0.32c-1.09 0.09-1.65 0.77-1.48 1.73l1.61 7.69c0.26 1.41 1.48 2.43 2.72 2.43h13.96c1.32 0 2.56-1.04 2.8-2.43l1.57-7.69c0.19-1.04-0.46-1.65-1.54-1.73z" strokeWidth={.3889} />
        <path d="m29.51 32.62c0.28 1.76 1.82 3 3.4 3h14c1.64 0 3.21-1.28 3.5-3" strokeWidth={.2222} />
        <circle cx={30.95} cy={41.42} r={4.855} strokeWidth={.2639} />
        <circle cx={49.05} cy={41.42} r={4.855} strokeWidth={.2639} />
        <rect x={38.4} y={42.78} width={3.203} height={1.074} rx={.5371} strokeWidth={.2639} />
        <path d="m15.54 58.36c0.7 0.39 1.69 0.58 2.34 0.51 1.23-0.12 2.32-1.3 3.4-3.35 1.23-2.36 2.28-5.18 3.3-7.24 0.7-1.49 1.91-2.11 3.23-2.11 1.36 0 1.98 0.47 3.16 0.47 1.35 0 2.01-0.61 3.5-0.61h10.94c1.54 0 2.26 0.61 3.62 0.61 1.24 0 1.85-0.47 3.16-0.47 1.45 0 2.52 0.61 3.23 2.04 1.13 2.26 2.18 4.95 3.4 7.28 1.08 2.07 2.31 3.26 3.43 3.38 0.8 0.08 1.59-0.13 2.21-0.51" strokeWidth={.4444} />
        <path d="m18.69 58.44c0.21-0.24 0.39-0.59 0.56-0.96 2.05-4.53 4.17-8.7 6.01-10.44" strokeWidth={.2361} />
        <path d="m61.33 58.44c-0.21-0.24-0.41-0.62-0.58-0.96-2.05-4.56-4.19-8.79-6.05-10.51" strokeWidth={.2361} />
      </g>
      <g className={styles.dots}>
        <circle cx={37.54} cy={36.89} r={.3164} />
        <circle cx={38.78} cy={36.89} r={.3164} />
        <circle cx={39.96} cy={36.89} r={.3164} />
        <circle cx={41.17} cy={36.89} r={.3164} />
        <circle cx={42.42} cy={36.89} r={.3164} />
        <circle cx={38.21} cy={37.92} r={.3164} />
        <circle cx={39.34} cy={37.92} r={.3164} />
        <circle cx={40.54} cy={37.92} r={.3164} />
        <circle cx={41.73} cy={37.92} r={.3164} />
        <circle cx={38.78} cy={38.83} r={.3086} />
        <circle cx={39.96} cy={38.83} r={.3086} />
        <circle cx={41.17} cy={38.83} r={.3086} />
      </g>

      <g className={styles.line}>
        {trigger("l2", state?.lt ?? 0, "M18.2 21.4 C18.8 18.9 20.4 18.1 22.1 18.1 C23.8 18.1 25.4 18.9 26 21.4 Z")}
        {trigger("r2", state?.rt ?? 0, "M53.9 21.4 C54.5 18.9 56.1 18.1 57.8 18.1 C59.5 18.1 61.1 18.9 61.7 21.4 Z")}
        {part("l1", <path d="m26.68 23.18c-0.05-0.52-0.24-0.87-0.93-1.1-0.62-0.2-1.37-0.27-1.96-0.24-1.83 0.1-4.04 0.64-5.14 1.41-0.76 0.52-0.95 1.21-1.02 1.8" strokeWidth={.4167} />)}
        {part("r1", <path d="m53.32 23.18c0.05-0.52 0.24-0.87 0.93-1.1 0.62-0.2 1.37-0.27 1.96-0.24 1.83 0.1 4.04 0.64 5.14 1.41 0.76 0.52 0.89 1.21 0.96 1.8" strokeWidth={.4167} />)}
        {part("select", <path d="m26.31 25.12c0.34 0 0.61 0.22 0.66 0.54l0.24 1.42c0.06 0.4-0.26 0.7-0.7 0.7-0.38 0-0.64-0.23-0.7-0.57l-0.24-1.37c-0.08-0.4 0.26-0.72 0.74-0.72z" strokeWidth={.2639} />)}
        {part("start", <path d="m53.53 25.12c-0.34 0-0.61 0.22-0.67 0.54l-0.23 1.42c-0.06 0.4 0.26 0.7 0.76 0.7 0.34 0 0.62-0.23 0.68-0.57l0.26-1.37c0.08-0.4-0.32-0.72-0.8-0.72z" strokeWidth={.2639} />)}
        {part("guide", <circle cx={40} cy={40.9} r={1.05} strokeWidth={0.26} />)}
        {part("dpadUp", <path d="m22.59 27.66c-0.69-0.07-1.33-0.07-1.94 0-0.49 0.05-0.84 0.43-0.83 0.92l0.03 1.47c0 0.24 0.09 0.46 0.26 0.63l1.08 1.03c0.23 0.22 0.71 0.22 0.94 0l1.08-1.06c0.15-0.15 0.23-0.36 0.23-0.58l0.04-1.46c0.02-0.52-0.36-0.91-0.89-0.95z" strokeWidth={.3472} />)}
        {part("dpadLeft", <path d="m20.59 32.26-1.18-1.07c-0.16-0.14-0.37-0.22-0.59-0.22l-1.48-0.01c-0.53 0-0.95 0.37-0.97 0.88-0.03 0.62-0.02 1.23 0.02 1.8 0.04 0.54 0.45 0.92 0.95 0.92h1.46c0.23 0 0.45-0.08 0.62-0.23l1.13-1.04c0.23-0.21 0.25-0.82 0.04-1.03z" strokeWidth={.3472} />)}
        {part("dpadRight", <path d="m22.72 32.26 1.18-1.07c0.16-0.14 0.37-0.22 0.59-0.22l1.42-0.01c0.54 0 0.96 0.35 0.99 0.85 0.04 0.65 0.03 1.28-0.01 1.84-0.03 0.54-0.45 0.91-0.95 0.91h-1.46c-0.23 0-0.45-0.08-0.62-0.23l-1.14-1.06c-0.22-0.21-0.21-0.81 0-1.01z" strokeWidth={.3472} />)}
        {part("dpadDown", <path d="m21.22 33.81-1.13 1.15c-0.15 0.15-0.23 0.35-0.23 0.57l-0.03 1.35c-0.01 0.54 0.38 0.98 0.91 1.02 0.66 0.05 1.29 0.04 1.87 0 0.53-0.04 0.89-0.46 0.87-0.97l-0.03-1.39c0-0.21-0.09-0.41-0.23-0.56l-1.11-1.15c-0.2-0.2-0.7-0.19-0.89-0.02z" strokeWidth={.3472} />)}
        {part("north", <circle cx={58.52} cy={28.86} r={2.064} strokeWidth={.3056} />)}
        {part("west", <circle cx={54.43} cy={32.79} r={2.023} strokeWidth={.3194} />)}
        {part("east", <circle cx={62.58} cy={32.79} r={2.023} strokeWidth={.3194} />)}
        {part("south", <ellipse cx={58.52} cy={36.81} rx={2.16} ry={2.033} strokeWidth={.3194} />)}
      </g>

      {(Object.keys(FACE_POSITIONS) as (keyof typeof FACE_POSITIONS)[]).map((button) => (
        <PlayStationSymbol key={button} button={button} x={FACE_POSITIONS[button][0]} y={FACE_POSITIONS[button][1]} color={PLAYSTATION_COLORS[button]} r={0.95} />
      ))}

      <g className={styles.line}>
        {stick("l3", state?.lx ?? 0, state?.ly ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={30.95} cy={41.24} r={3.581} strokeWidth={.4028} />
            <circle cx={30.95} cy={41.24} r={2.488} strokeWidth={.2222} />
          </>
        ))}
        {stick("r3", state?.rx ?? 0, state?.ry ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={49.05} cy={41.24} r={3.545} strokeWidth={.4028} />
            <circle cx={49.07} cy={41.24} r={2.488} strokeWidth={.2222} />
          </>
        ))}
      </g>
    </svg>
  );
}
