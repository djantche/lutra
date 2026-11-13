import { useTranslation } from "react-i18next";
import { buttonLabel } from "../../lib/buttonLabels";
import type { ControllerButton, ControllerFamily, PadSnapshot } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import { ControllerSvg } from "./ControllerSvg";
import styles from "./GamepadVisual.module.scss";

const STICK_TRAVEL = 11;

interface GamepadVisualProps {
  state: PadSnapshot | null;
  family: ControllerFamily;
}

/**
 * The controller drawn live: what is pressed lights up, sticks and triggers move.
 * PlayStation controllers get a line drawing of a DualShock 4; others a schematic until they have one too.
 */
export function GamepadVisual({ state, family }: GamepadVisualProps) {
  if (family === "playstation") return <ControllerSvg state={state} />;
  return <SchematicPad state={state} family={family} />;
}

function SchematicPad({ state, family }: GamepadVisualProps) {
  const { t } = useTranslation();
  const on = (button: ControllerButton) => state?.pressed.includes(button) ?? false;
  const cls = (button: ControllerButton) => cx(styles.button, on(button) && styles.on);
  const label = (button: ControllerButton) => buttonLabel(button, family, t);

  const stick = (cx0: number, cy0: number, x: number, y: number, button: ControllerButton) => (
    <g>
      <circle cx={cx0} cy={cy0} r={22} className={styles.well} />
      <circle
        cx={cx0 + x * STICK_TRAVEL}
        cy={cy0 + y * STICK_TRAVEL}
        r={14}
        className={cx(styles.stick, on(button) && styles.on)}
      />
    </g>
  );

  const trigger = (x: number, value: number, button: ControllerButton) => (
    <g>
      <rect x={x} y={4} width={62} height={14} rx={7} className={styles.well} />
      <rect x={x} y={4} width={62 * value} height={14} rx={7} className={styles.fill} />
      <text x={x + 31} y={14.5} className={cx(styles.tiny, on(button) && styles.lit)}>
        {label(button)}
      </text>
    </g>
  );

  const face = (x: number, y: number, button: ControllerButton) => (
    <g>
      <circle cx={x} cy={y} r={10.5} className={cls(button)} />
      <text x={x} y={y + 4} className={cx(styles.glyph, on(button) && styles.lit)}>
        {label(button)}
      </text>
    </g>
  );

  return (
    <svg className={styles.pad} viewBox="-6 0 372 226" role="img" aria-label={t("controllers.liveView")}>
      {trigger(58, state?.lt ?? 0, "l2")}
      {trigger(240, state?.rt ?? 0, "r2")}
      <rect x={52} y={24} width={74} height={11} rx={5.5} className={cls("l1")} />
      <rect x={234} y={24} width={74} height={11} rx={5.5} className={cls("r1")} />

      <path
        className={styles.body}
        d="M72 40 H288 C326 40 346 66 352 104 L360 168 C364 204 334 222 310 202 L278 172 H82 L50 202 C26 222 -4 204 0 168 L8 104 C14 66 34 40 72 40 Z"
      />

      {stick(110, 92, state?.lx ?? 0, state?.ly ?? 0, "l3")}
      {stick(222, 146, state?.rx ?? 0, state?.ry ?? 0, "r3")}

      <g>
        <rect x={130} y={124} width={16} height={18} rx={3} className={cls("dpadUp")} />
        <rect x={130} y={158} width={16} height={18} rx={3} className={cls("dpadDown")} />
        <rect x={111} y={142} width={18} height={16} rx={3} className={cls("dpadLeft")} />
        <rect x={147} y={142} width={18} height={16} rx={3} className={cls("dpadRight")} />
      </g>

      {face(258, 72, "north")}
      {face(282, 94, "east")}
      {face(258, 116, "south")}
      {face(234, 94, "west")}

      <rect x={156} y={74} width={18} height={9} rx={4.5} className={cls("select")} />
      <rect x={186} y={74} width={18} height={9} rx={4.5} className={cls("start")} />
      <circle cx={180} cy={108} r={9} className={cls("guide")} />
    </svg>
  );
}
