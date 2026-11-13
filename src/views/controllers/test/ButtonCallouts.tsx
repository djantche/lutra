import { useTranslation } from "react-i18next";
import { ButtonGlyph } from "../../../components/ButtonGlyph/ButtonGlyph";
import { keyLabel, outputLabel } from "../../../lib/buttonLabels";
import {
  keyFor,
  mappedButton,
  type ControllerButton,
  type ControllerFamily,
  type ControllerProfile,
} from "../../../lib/controllers";
import { cx } from "../../../lib/cx";
import styles from "./ControllerTestScreen.module.scss";

interface ButtonCalloutsProps {
  side: "left" | "right";
  buttons: ControllerButton[];
  profile: ControllerProfile;
  family: ControllerFamily;
  pressed: ControllerButton[];
  /** When given, each line is a button that picks its physical button. */
  onPick?: (button: ControllerButton) => void;
  /** The button being remapped right now. */
  active?: ControllerButton | null;
}

/** For each button: its name on the controller, and what the game receives (lit while pressed). */
export function ButtonCallouts({ side, buttons, profile, family, pressed, onPick, active = null }: ButtonCalloutsProps) {
  const { t } = useTranslation();

  return (
    <ul className={cx(styles.callouts, side === "right" && styles.right)}>
      {buttons.map((button) => {
        const target = profile.output === "none" ? button : mappedButton(profile, button);
        const key = profile.output === "none" ? undefined : keyFor(profile, button);
        const changed = key !== undefined || target !== button;
        const content = (
          <>
            <ButtonGlyph button={button} family={family} />
            <span className={styles.arrow}>→</span>
            <span className={cx(styles.target, changed && styles.changed)}>
              {key !== undefined
                ? keyLabel(key, t)
                : target === null
                  ? t("controllers.disabled")
                  : outputLabel(target, profile.output, t)}
            </span>
          </>
        );
        const classes = cx(styles.callout, pressed.includes(button) && styles.lit, active === button && styles.picking);
        return (
          <li key={button}>
            {onPick ? (
              <button type="button" data-focusable className={cx(classes, styles.pickable)} onClick={() => onPick(button)}>
                {content}
              </button>
            ) : (
              <div className={classes}>{content}</div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
