import type { CSSProperties, ReactElement } from "react";
import { useTranslation } from "react-i18next";
import { buttonLabel } from "../../lib/buttonLabels";
import type { ControllerButton, ControllerFamily } from "../../lib/controllers";
import { cx } from "../../lib/cx";
import { PLAYSTATION_COLORS, XBOX_FACE } from "../GamepadVisual/padParts";
import styles from "./ButtonGlyph.module.scss";

type FaceButton = keyof typeof PLAYSTATION_COLORS;

const isFace = (button: ControllerButton): button is FaceButton => button in PLAYSTATION_COLORS;

// The PlayStation symbols, drawn in a 24-unit box with one round stroke.
const SYMBOLS: Record<FaceButton, ReactElement> = {
  north: <path d="M12 5.5 19 18H5Z" />,
  west: <rect x="6" y="6" width="12" height="12" rx="1" />,
  east: <circle cx="12" cy="12" r="7" />,
  south: <path d="M6.5 6.5 17.5 17.5M17.5 6.5 6.5 17.5" />,
};

/** What gives a face button its colour: PlayStation's symbols and Xbox's letters. Others stay plain. */
function faceColor(button: FaceButton, family: ControllerFamily): string | undefined {
  if (family === "playstation") return PLAYSTATION_COLORS[button];
  if (family === "xbox") return XBOX_FACE[button].color;
  return undefined;
}

/**
 * A button as printed on the controller. Face buttons are round badges in the maker's colour
 * (PlayStation symbols, Xbox letters); every other button is a pill with its name.
 */
export function ButtonGlyph({ button, family }: { button: ControllerButton; family: ControllerFamily }) {
  const { t } = useTranslation();
  const label = buttonLabel(button, family, t);

  if (!isFace(button)) return <kbd className={styles.glyph}>{label}</kbd>;

  const color = faceColor(button, family);
  return (
    <span
      className={cx(styles.glyph, styles.face)}
      style={color ? ({ "--face": color } as CSSProperties) : undefined}
      role="img"
      aria-label={label}
      title={label}
    >
      {family === "playstation" ? (
        <svg viewBox="0 0 24 24" className={styles.symbol} aria-hidden="true">
          {SYMBOLS[button]}
        </svg>
      ) : (
        label
      )}
    </span>
  );
}
