import { useTranslation } from "react-i18next";
import photo from "../../assets/controllers/playstation.webp";
import type { ControllerButton, PadSnapshot } from "../../lib/controllers";
import styles from "./ControllerPhoto.module.scss";

// Coordinates in the render's own pixels (1536 × 1024): face buttons measured from their
// coloured symbols, the rest on a graduated zoom, all checked by drawing them over the render.
const WIDTH = 1536;
const HEIGHT = 1024;
const STICK_TRAVEL = 58;

type Shape = { cx: number; cy: number; r: number } | { x: number; y: number; w: number; h: number; rx: number };

const circle = (cx: number, cy: number, r: number): Shape => ({ cx, cy, r });
const rect = (x: number, y: number, w: number, h: number, rx: number): Shape => ({ x, y, w, h, rx });

const BUTTONS: Partial<Record<ControllerButton, Shape>> = {
  north: circle(1253, 252, 48),
  west: circle(1148, 368, 48),
  east: circle(1359, 368, 48),
  south: circle(1256, 478, 48),
  guide: circle(768, 604, 38),
  dpadUp: rect(248, 244, 84, 92, 16),
  dpadDown: rect(250, 398, 83, 104, 16),
  dpadLeft: rect(162, 332, 100, 81, 16),
  dpadRight: rect(318, 332, 98, 80, 16),
  select: rect(432, 180, 38, 82, 19),
  start: rect(1062, 180, 38, 82, 19),
  l1: rect(205, 95, 210, 33, 16),
  r1: rect(1122, 95, 210, 33, 16),
};

const TRIGGERS = { l2: rect(210, 45, 200, 50, 22), r2: rect(1127, 45, 200, 50, 22) };
const STICKS = { l3: { cx: 530, cy: 598 }, r3: { cx: 1003, cy: 598 } };

function ShapeView({ shape, className, opacity }: { shape: Shape; className: string; opacity?: number }) {
  return "r" in shape ? (
    <circle cx={shape.cx} cy={shape.cy} r={shape.r} className={className} opacity={opacity} />
  ) : (
    <rect x={shape.x} y={shape.y} width={shape.w} height={shape.h} rx={shape.rx} className={className} opacity={opacity} />
  );
}

/** A DualShock 4 render with the live state drawn over it: pressed buttons, triggers and sticks. */
export function ControllerPhoto({ state }: { state: PadSnapshot | null }) {
  const { t } = useTranslation();
  const pressed = (button: ControllerButton) => state?.pressed.includes(button) ?? false;

  const stick = (button: "l3" | "r3", x: number, y: number) => {
    const { cx, cy } = STICKS[button];
    const moved = Math.hypot(x, y) > 0.04;
    const tip = { x: cx + x * STICK_TRAVEL, y: cy + y * STICK_TRAVEL };
    return (
      <g key={button}>
        {pressed(button) && <circle cx={cx} cy={cy} r={92} className={styles.pressed} />}
        {moved && <line x1={cx} y1={cy} x2={tip.x} y2={tip.y} className={styles.vector} />}
        <circle cx={tip.x} cy={tip.y} r={moved ? 16 : 0} className={styles.tip} />
      </g>
    );
  };

  return (
    <svg className={styles.photo} viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img" aria-label={t("controllers.liveView")}>
      <image href={photo} width={WIDTH} height={HEIGHT} />

      <ShapeView shape={TRIGGERS.l2} className={styles.trigger} opacity={state?.lt ?? 0} />
      <ShapeView shape={TRIGGERS.r2} className={styles.trigger} opacity={state?.rt ?? 0} />

      {Object.entries(BUTTONS).map(
        ([button, shape]) =>
          pressed(button as ControllerButton) && <ShapeView key={button} shape={shape} className={styles.pressed} />,
      )}

      {stick("l3", state?.lx ?? 0, state?.ly ?? 0)}
      {stick("r3", state?.rx ?? 0, state?.ry ?? 0)}
    </svg>
  );
}
