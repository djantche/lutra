import type { ControllerFamily, PadModel, PadSnapshot } from "../../lib/controllers";
import { ControllerSvg } from "./ControllerSvg";
import { DualSenseSvg } from "./DualSenseSvg";
import { FightstickSvg } from "./FightstickSvg";
import { GenericPadSvg } from "./GenericPadSvg";
import { SteamDeckSvg } from "./SteamDeckSvg";

interface GamepadVisualProps {
  /** What is pressed, or null for the controller at rest (a plain picture). */
  state: PadSnapshot | null;
  family: ControllerFamily;
  /** Which controller it looks like; the ones without a drawing of their own get a standard gamepad. */
  model?: PadModel;
}

/**
 * The controller drawn live: what is pressed lights up, sticks and triggers move.
 * The DualShock 4, DualSense, Steam Deck and arcade sticks have a line drawing of their own; every
 * other controller gets a standard gamepad, its buttons named the way its maker does (`family`).
 */
export function GamepadVisual({ state, family, model }: GamepadVisualProps) {
  if (model === "dualsense") return <DualSenseSvg state={state} />;
  if (model === "steamdeck") return <SteamDeckSvg state={state} />;
  if (model === "fightstick") return <FightstickSvg state={state} />;
  if (model === "dualshock4" || (model === undefined && family === "playstation")) return <ControllerSvg state={state} />;
  return <GenericPadSvg state={state} family={family} />;
}
