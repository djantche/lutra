import type { TFunction } from "i18next";
import type { ControllerButton, ControllerFamily, OutputKind } from "./controllers";

type Labels = Partial<Record<ControllerButton, string>>;

// What each maker prints on its buttons. Positions are the same everywhere; names are not.
const BY_FAMILY: Record<ControllerFamily, Labels> = {
  playstation: {
    south: "✕",
    east: "○",
    west: "□",
    north: "△",
    l1: "L1",
    r1: "R1",
    l2: "L2",
    r2: "R2",
    l3: "L3",
    r3: "R3",
    select: "Share",
    start: "Options",
    guide: "PS",
  },
  xbox: {
    south: "A",
    east: "B",
    west: "X",
    north: "Y",
    l1: "LB",
    r1: "RB",
    l2: "LT",
    r2: "RT",
    l3: "LS",
    r3: "RS",
    select: "View",
    start: "Menu",
    guide: "Xbox",
  },
  nintendo: {
    south: "B",
    east: "A",
    west: "Y",
    north: "X",
    l1: "L",
    r1: "R",
    l2: "ZL",
    r2: "ZR",
    l3: "LS",
    r3: "RS",
    select: "−",
    start: "+",
    guide: "Home",
  },
  generic: {
    south: "A",
    east: "B",
    west: "X",
    north: "Y",
    l1: "L1",
    r1: "R1",
    l2: "L2",
    r2: "R2",
    l3: "L3",
    r3: "R3",
    select: "Select",
    start: "Start",
    guide: "Guide",
  },
};

/** Short name printed on the button (✕, A, LB...). D-pad directions get a translated word. */
export function buttonLabel(button: ControllerButton, family: ControllerFamily, t: TFunction): string {
  return BY_FAMILY[family][button] ?? t(`controllers.buttons.${button}`);
}

/** The emulated controller's own name for a button, as the game will show it. */
export function outputLabel(button: ControllerButton, output: OutputKind, t: TFunction): string {
  return buttonLabel(button, output === "ds4" ? "playstation" : "xbox", t);
}

/** A key or mouse button as the person knows it: letters and F-keys as printed, the rest translated. */
export function keyLabel(name: string, t: TFunction): string {
  return /^([a-z]|[0-9]|f\d{1,2})$/.test(name) ? name.toUpperCase() : t(`controllers.keys.${name}`, { defaultValue: name });
}
