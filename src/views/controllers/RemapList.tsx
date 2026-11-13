import { useTranslation } from "react-i18next";
import { ButtonGlyph } from "../../components/ButtonGlyph/ButtonGlyph";
import { Select } from "../../components/Select/Select";
import { buttonLabel, keyLabel, outputLabel } from "../../lib/buttonLabels";
import { BUTTONS, type ControllerButton, type ControllerFamily, type ControllerProfile } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import styles from "./Controllers.module.scss";
import { useControllerKeys } from "./useControllerKeys";

const SAME = "same";
const OFF = "off";
const KEY = "key:";

interface RemapListProps {
  profile: ControllerProfile;
  family: ControllerFamily;
  /** Buttons held right now: their row lights up, so pressing a button finds its line. */
  pressed: ControllerButton[];
  onChange: (patch: Pick<ControllerProfile, "remap" | "keys">) => void;
}

/** One line per physical button: what the game receives when it is pressed (a button, or a key). */
export function RemapList({ profile, family, pressed, onChange }: RemapListProps) {
  const { t } = useTranslation();
  const keys = useControllerKeys();
  const target = (button: ControllerButton) => outputLabel(button, profile.output, t);

  const set = (button: ControllerButton, value: string) => {
    const remap = { ...profile.remap };
    const bound = { ...profile.keys };
    delete remap[button];
    delete bound[button];
    if (value.startsWith(KEY)) bound[button] = value.slice(KEY.length);
    else if (value === OFF) remap[button] = null;
    else if (value !== SAME) remap[button] = value as ControllerButton;
    onChange({ remap, keys: bound });
  };

  // With a keyboard-only output there is no gamepad to send a button to.
  const gamepad = profile.output !== "keyboard";

  return (
    <div className={styles.remap}>
      {BUTTONS.map((button) => {
        const key = profile.keys[button];
        const mapped = profile.remap[button];
        const value = key ? KEY + key : mapped === undefined ? SAME : mapped === null ? OFF : mapped;
        return (
          <div key={button} className={cx(styles.remapRow, pressed.includes(button) && styles.held)}>
            <span className={styles.physical}>
              <ButtonGlyph button={button} family={family} />
              {t(`controllers.buttons.${button}`)}
            </span>
            <div className={styles.remapSelect}>
              <Select
                ariaLabel={t("controllers.remapFor", { button: buttonLabel(button, family, t) })}
                value={value}
                onChange={(next) => set(button, next)}
                options={[
                  ...(gamepad
                    ? [{ value: SAME, label: t("controllers.same", { button: target(button) }) }]
                    : []),
                  ...(gamepad
                    ? BUTTONS.filter((other) => other !== button).map((other) => ({
                        value: other,
                        label: `${target(other)} · ${t(`controllers.buttons.${other}`)}`,
                      }))
                    : []),
                  ...keys.map((info) => ({
                    value: KEY + info.name,
                    label: keyLabel(info.name, t),
                    hint: t(`controllers.keyGroups.${info.group}`),
                  })),
                  { value: OFF, label: t("controllers.disabled") },
                ]}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}
