import { useId, type CSSProperties, type ReactElement } from "react";
import { useTranslation } from "react-i18next";
import { buttonLabel } from "../../lib/buttonLabels";
import type { ControllerButton, ControllerFamily, PadSnapshot } from "../../lib/controllers";
import { padParts, XBOX_FACE } from "./padParts";
import styles from "./ControllerSvg.module.scss";

// A standard gamepad drawn as line art (from `game-controller.svg`), for every controller that has
// no drawing of its own: see `ControllerSvg` for how these are used. The drawing is in a 512-unit
// box, so its numbers are in those units. Its two bumpers are the bumps at the top, closed along
// the crease under them; the triggers (not part of it) are arches above the bumps.

const STICK_TRAVEL = 13;
const DPAD = { x: 181.7, y: 276.2, reach: 60 };

const BODY =
  "m438.1 126c-7.808-22.21-15.3-47.55-70.27-47.55-13.5 0-19.46 2.112-27.07 14.78-1.728 2.944-6.016 4.352-9.92 4.352h-149.2c-4.352 0-8.64-1.088-10.37-4.16-7.936-12.86-14.08-15.17-27.71-15.17-53.7 0-61.76 23.94-69.31 46.85-6.528 6.848-12.48 14.66-16.7 24.32-22.98 52.16-51.84 141.8-51.84 196 0 65.98 36.54 88.06 53.82 88.06 12.54 0 22.85-9.024 46.98-40.13 19.14-24.19 26.43-42.62 55.17-42.62h184.5c28.86 0 36.54 12.29 53.76 35.26 26.37 34.37 38.46 47.68 51.78 47.68 22.02 0 54.34-31.17 54.34-86.53 0.64-54.91-30.53-152.4-52.16-197.8-4.032-9.28-9.664-16.83-15.81-23.42z";
const CREASE =
  "m74.3 125.4c21.06-13.31 54.72-22.27 75.71-23.23 6.464-0.128 12.74 1.984 16.51 6.528l20.1 23.3c3.968 4.8 9.536 7.936 15.42 7.936l107.3 0.256c6.464 0 11.58-2.752 15.68-7.552l19.14-22.46c4.096-4.992 10.75-7.552 17.22-7.232 25.79 1.472 57.34 11.07 76.54 23.42";
const GRIP =
  "m59.97 432.4c15.74-25.34 40.13-55.81 57.34-73.98 10.05-10.24 21.5-17.73 34.94-17.73h203.5c13.12 0 24.19 5.696 34.3 15.36 20.48 20.48 46.85 55.04 60.8 76.67";
const DPAD_CROSS =
  "m210.4 261.5h-13.95v-14.4c0-5.184-4.096-9.6-9.216-9.6h-10.3c-5.248 0-9.344 4.288-9.344 9.6v14.59h-14.72c-5.184 0-9.408 4.032-9.408 9.472v10.05c0 5.312 4.288 9.6 9.408 9.6h14.66v14.53c0 5.184 4.288 9.088 9.28 9.088h10.18c5.248 0 9.344-4.096 9.344-9.088v-14.66h14.02c5.376 0 10.11-4.288 10.11-9.728v-9.728c0.128-5.44-4.8-9.728-10.05-9.728z";
const BUMPER_LEFT =
  "M74.3 125.4C95.4 112.1 129 103.1 150 102.2C156.5 102 162.8 104.2 166.5 108.7L171.3 93.4C163.3 80.6 157.2 78.3 143.6 78.3C89.9 78.3 81.8 102.2 74.3 125.1Z";
const BUMPER_RIGHT =
  "M438.1 126C430.3 103.8 422.8 78.5 367.8 78.5C354.3 78.5 348.4 80.6 340.8 93.2L344.2 110.2C348.3 105.2 354.9 102.6 361.4 102.9C387.2 104.4 418.7 114 437.9 126.4Z";
const TRIGGER_LEFT = "M84 100 C96 66 124 52 152 58 C166 62 174 76 177 94 Z";
const TRIGGER_RIGHT = "M428 100 C416 66 388 52 360 58 C346 62 338 76 335 94 Z";

const FACE: Record<"north" | "west" | "east" | "south", { x: number; y: number; r: number }> = {
  north: { x: 392.5, y: 152.3, r: 18.3 },
  west: { x: 356.4, y: 187.1, r: 18.2 },
  east: { x: 427.3, y: 186.9, r: 18.15 },
  south: { x: 393, y: 223.4, r: 18.5 },
};

const XBOX_COLORS = {
  north: XBOX_FACE.north.color,
  west: XBOX_FACE.west.color,
  east: XBOX_FACE.east.color,
  south: XBOX_FACE.south.color,
};

/** The pad, live when given a state: pressed buttons fill, sticks move, triggers fill with their pull. */
export function GenericPadSvg({ state, family }: { state: PadSnapshot | null; family: ControllerFamily }) {
  const { t } = useTranslation();
  const clips = useId().replace(/:/g, "");
  const { part, stick, trigger } = padParts(state, family === "xbox" ? XBOX_COLORS : {});

  /** A quarter of the cross, cut along the diagonals, filled while its direction is pressed. */
  const arm = (button: "dpadUp" | "dpadDown" | "dpadLeft" | "dpadRight", points: string) => (
    <>
      <clipPath id={`${clips}-${button}`}>
        <polygon points={points} />
      </clipPath>
      {part(
        button,
        <g clipPath={`url(#${clips}-${button})`}>
          <path d={DPAD_CROSS} strokeWidth={3.6} />
        </g>,
      )}
    </>
  );
  const { x: cx, y: cy, reach } = DPAD;

  const face = (button: keyof typeof FACE) => {
    const { x, y, r } = FACE[button];
    return (
      <g key={button}>
        {part(button, <circle cx={x} cy={y} r={r} strokeWidth={3.6} />)}
        <text x={x} y={y} className={styles.label} style={{ fontSize: 22 } as CSSProperties}>
          {buttonLabel(button, family, t)}
        </text>
      </g>
    );
  };

  const pill = (button: ControllerButton, shape: ReactElement) => part(button, shape);

  return (
    <svg className={styles.pad} viewBox="0 44 512 400" role="img" aria-label={t("controllers.liveView")}>
      <g className={styles.line}>
        {trigger("l2", state?.lt ?? 0, TRIGGER_LEFT)}
        {trigger("r2", state?.rt ?? 0, TRIGGER_RIGHT)}
        {part("l1", <path d={BUMPER_LEFT} strokeWidth={2.8} />)}
        {part("r1", <path d={BUMPER_RIGHT} strokeWidth={2.8} />)}

        <path d={BODY} strokeWidth={4.27} />
        <path d={CREASE} strokeWidth={2.84} />
        <path d="m80.26 110.9c23.94-15.62 62.4-19.14 91.97-15.23" strokeWidth={3.3} />
        <path d="m431.9 111.4c-24.19-15.81-62.53-18.88-91.58-15.68" strokeWidth={3.3} />
        <path d={GRIP} strokeWidth={2.33} />

        <circle cx={121.4} cy={186.5} r={44.81} strokeWidth={2.67} />
        <circle cx={322.4} cy={275.6} r={46.5} strokeWidth={2.67} />
        <circle cx={181.7} cy={276.2} r={46.95} strokeWidth={2.67} />
        <path d={DPAD_CROSS} strokeWidth={3.59} />
        {arm("dpadUp", `${cx},${cy} ${cx - reach},${cy - reach} ${cx + reach},${cy - reach}`)}
        {arm("dpadDown", `${cx},${cy} ${cx - reach},${cy + reach} ${cx + reach},${cy + reach}`)}
        {arm("dpadLeft", `${cx},${cy} ${cx - reach},${cy - reach} ${cx - reach},${cy + reach}`)}
        {arm("dpadRight", `${cx},${cy} ${cx + reach},${cy - reach} ${cx + reach},${cy + reach}`)}

        {pill("select", <rect x={206.9} y={177.7} width={28.84} height={17.39} rx={8.693} strokeWidth={2.96} />)}
        {pill("start", <rect x={277.2} y={177.3} width={28.51} height={17.39} rx={8.693} strokeWidth={2.96} />)}
        {pill("guide", <rect x={241.2} y={210.3} width={29.68} height={14.6} rx={7.299} strokeWidth={2.91} />)}

        {(Object.keys(FACE) as (keyof typeof FACE)[]).map(face)}

        {stick("l3", state?.lx ?? 0, state?.ly ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={121.7} cy={186} r={33.74} strokeWidth={4.27} />
            <circle cx={121.6} cy={185.8} r={25.05} strokeWidth={2.23} />
          </>
        ))}
        {stick("r3", state?.rx ?? 0, state?.ry ?? 0, STICK_TRAVEL, (
          <>
            <circle cx={322.7} cy={274.3} r={34.34} strokeWidth={4.27} />
            <circle cx={322.7} cy={274} r={26.01} strokeWidth={2.33} />
          </>
        ))}
      </g>
    </svg>
  );
}
