import { useTranslation } from "react-i18next";
import type { CSSProperties } from "react";
import type { ControllerButton, PadSnapshot } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import { padParts, XBOX_FACE } from "./padParts";
import styles from "./ControllerSvg.module.scss";

// An arcade stick drawn as line art (from `arcade-fightstick.svg`), seen from above: a lever on
// the left, two rows of four buttons on the right and three system buttons on the top edge. It is
// not one maker's stick: they all share this plan, and the buttons are the ones every pad has, in
// the usual order of a stick made for Xbox (X Y RB LB over A B RT LT). The drawing is in a 512-unit
// box, so its numbers are in those units.

/** Where the lever turns: the middle of its base, halfway between the two rows of buttons. */
const LEVER = { x: 112, y: 267 } as const;
/** How far the ball goes from there: the base is 46 units wide, the ball 29. */
const LEVER_TRAVEL = 14;
/** The lever follows the left stick when the stick sends it, and the D-pad otherwise. */
const LEVER_DEADZONE = 0.2;

/** Each button's inner circle (the part that fills), then the ring around it. */
interface Round {
  button: ControllerButton;
  x: number;
  y: number;
  inner: number;
  ring: number;
}
const BUTTONS: Round[] = [
  { button: "west", x: 225.3, y: 233.2, inner: 23.1, ring: 29.4 },
  { button: "north", x: 288.7, y: 233.2, inner: 23.1, ring: 29.4 },
  { button: "r1", x: 351.7, y: 233.2, inner: 22.6, ring: 29.4 },
  { button: "l1", x: 415.2, y: 233.2, inner: 23.3, ring: 29.7 },
  { button: "south", x: 224.9, y: 301.7, inner: 23.4, ring: 29.9 },
  { button: "east", x: 288.7, y: 301.7, inner: 23, ring: 29.9 },
  { button: "r2", x: 351.7, y: 301.7, inner: 22.7, ring: 29.4 },
  { button: "l2", x: 415.2, y: 301.7, inner: 23.4, ring: 29.9 },
];

const FACE_LABELS: Partial<Record<ControllerButton, string>> = {
  west: XBOX_FACE.west.label,
  north: XBOX_FACE.north.label,
  south: XBOX_FACE.south.label,
  east: XBOX_FACE.east.label,
};
const FACE_COLORS = {
  west: XBOX_FACE.west.color,
  north: XBOX_FACE.north.color,
  south: XBOX_FACE.south.color,
  east: XBOX_FACE.east.color,
};

/** The arcade stick, live when given a state: pressed buttons fill and the lever leans. */
export function FightstickSvg({ state }: { state: PadSnapshot | null }) {
  const { t } = useTranslation();
  const { pressed, part } = padParts(state, FACE_COLORS);

  const dpad = {
    x: Number(pressed("dpadRight")) - Number(pressed("dpadLeft")),
    y: Number(pressed("dpadDown")) - Number(pressed("dpadUp")),
  };
  const analogue = Math.hypot(state?.lx ?? 0, state?.ly ?? 0) > LEVER_DEADZONE;
  const lever = analogue ? { x: state?.lx ?? 0, y: state?.ly ?? 0 } : dpad;
  // A lever pushed into a corner stays on the circle it can reach.
  const length = Math.hypot(lever.x, lever.y);
  const scale = length > 1 ? 1 / length : 1;
  const leaning = length > 0.05;

  return (
    <svg className={styles.pad} viewBox="0 84 512 346" role="img" aria-label={t("controllers.liveView")}>
      <g className={styles.line}>
        <rect x={6.325} y={97.44} width={499.3} height={317} rx={38} strokeWidth={4.45} />
        <path
          d="m16.51 142.1c0-13.57 10.88-24.32 23.81-24.32h431.4c13.18 0 24.13 10.75 24.13 24.32v233.3c0 15.23-12.35 27.07-26.62 27.07h-424.3c-15.36 0-28.35-12.48-28.35-28.1v-232.3z"
          strokeWidth={2.675}
        />
        <path
          d="m29.12 155.8c0-9.664 7.808-17.09 16.77-17.09h87.36c6.656 0 13.18 3.072 17.41 8.384l8.768 10.88c3.904 5.056 10.05 8.32 16.26 8.32h160.4c6.336 0 12.61-3.072 16.51-8.256l8.512-10.75c4.224-5.504 10.94-8.768 17.73-8.768h87.04c9.344 0 17.47 7.296 17.47 17.47l0.128 212.5c0 8.96-7.04 15.55-15.74 15.55h-422.7c-8.704 0-15.94-6.848-15.94-15.81v-212.5z"
          strokeWidth={2.675}
        />
        <path
          d="m127.7 117.8c8.96 0 17.41 4.48 22.46 11.9l14.27 20.48c3.072 4.864 8.32 7.744 13.7 7.744h155.1c5.44 0 10.82-2.752 13.89-7.488l14.66-20.86c5.056-7.296 13.5-11.78 22.34-11.78"
          strokeWidth={2.675}
        />
        <circle cx={45.72} cy={154.7} r={5.7} strokeWidth={2.5} />
        <circle cx={465.5} cy={154.7} r={5.7} strokeWidth={2.5} />
        <circle cx={46.15} cy={365.3} r={5.6} strokeWidth={2.5} />
        <circle cx={465.6} cy={365.3} r={5.6} strokeWidth={2.5} />

        {part("select", <rect x={194} y={130.3} width={27.7} height={16.4} rx={7.65} strokeWidth={2.85} />)}
        {part("guide", <rect x={242} y={130.3} width={27.95} height={16.4} rx={7.65} strokeWidth={2.85} />)}
        {part("start", <rect x={290} y={130.3} width={27.45} height={16.4} rx={7.65} strokeWidth={2.85} />)}

        {BUTTONS.map(({ button, x, y, inner, ring }) => (
          <g key={button}>
            <circle cx={x} cy={y} r={ring} strokeWidth={2.7} />
            {part(button, <circle cx={x} cy={y} r={inner} strokeWidth={2.7} />)}
            {FACE_LABELS[button] && (
              <text x={x} y={y} className={styles.label} style={{ fontSize: 30 } as CSSProperties}>
                {FACE_LABELS[button]}
              </text>
            )}
          </g>
        ))}

        {/* Seen from above, the lever is a ball over a round base: two closed rings that stay, and a
            cap of two circles that leans the way it is pushed, like a stick on any other pad. */}
        <circle cx={LEVER.x} cy={LEVER.y} r={46.5} strokeWidth={2.85} />
        <circle cx={LEVER.x} cy={LEVER.y} r={38} strokeWidth={2.5} />
        <g
          className={cx(styles.stick, leaning && styles.on)}
          style={{ transform: `translate(${lever.x * scale * LEVER_TRAVEL}px, ${lever.y * scale * LEVER_TRAVEL}px)` }}
        >
          <circle cx={LEVER.x} cy={LEVER.y} r={29} strokeWidth={3.3} />
          <circle cx={LEVER.x} cy={LEVER.y} r={19.5} strokeWidth={2.2} />
        </g>
      </g>
    </svg>
  );
}
