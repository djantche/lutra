import { useTranslation } from "react-i18next";
import east from "../../assets/controllers/playstation/east.webp";
import l1 from "../../assets/controllers/playstation/l1.webp";
import l2 from "../../assets/controllers/playstation/l2.webp";
import l3 from "../../assets/controllers/playstation/l3.webp";
import north from "../../assets/controllers/playstation/north.webp";
import r1 from "../../assets/controllers/playstation/r1.webp";
import r2 from "../../assets/controllers/playstation/r2.webp";
import r3 from "../../assets/controllers/playstation/r3.webp";
import south from "../../assets/controllers/playstation/south.webp";
import west from "../../assets/controllers/playstation/west.webp";
import { buttonLabel } from "../../lib/buttonLabels";
import type { ControllerButton, ControllerFamily } from "../../lib/controllers";
import styles from "./ButtonGlyph.module.scss";

// Buttons with a drawn icon; the others (D-pad directions, Share, Options, PS) stay as text.
const PLAYSTATION: Partial<Record<ControllerButton, string>> = { south, east, west, north, l1, l2, r1, r2, l3, r3 };

/** A button as printed on the controller: its icon when there is one, its name otherwise. */
export function ButtonGlyph({ button, family }: { button: ControllerButton; family: ControllerFamily }) {
  const { t } = useTranslation();
  const label = buttonLabel(button, family, t);
  const icon = family === "playstation" ? PLAYSTATION[button] : undefined;

  return icon ? <img className={styles.icon} src={icon} alt={label} title={label} draggable={false} /> : <kbd>{label}</kbd>;
}
