/**
 * Device-agnostic input actions. The UI only reacts to these, never to raw
 * keyboard or gamepad events, so new input sources (e.g. the Rust controller
 * module) can be plugged in without touching the screens.
 */
export type InputAction =
  | "up"
  | "down"
  | "left"
  | "right"
  | "confirm"
  | "back"
  | "tabPrev"
  | "tabNext"
  /** F11: swaps between the desktop and Big Picture modes. */
  | "toggleMode"
  /** The controller's Guide button (released): Big Picture's menu, or entering Big Picture. */
  | "menu"
  /** Options / Select: the settings of what is focused. */
  | "options"
  /** The Guide button held: the controller is being switched off. */
  | "powerOff";
