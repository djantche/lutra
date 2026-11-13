import { invoke, listen, type UnlistenFn } from "./host";
import { hasBackend } from "./backend";

export type ControllerButton =
  | "south"
  | "east"
  | "north"
  | "west"
  | "l1"
  | "r1"
  | "l2"
  | "r2"
  | "select"
  | "start"
  | "guide"
  | "l3"
  | "r3"
  | "dpadUp"
  | "dpadDown"
  | "dpadLeft"
  | "dpadRight";

export const BUTTONS: ControllerButton[] = [
  "south",
  "east",
  "west",
  "north",
  "l1",
  "r1",
  "l2",
  "r2",
  "l3",
  "r3",
  "select",
  "start",
  "guide",
  "dpadUp",
  "dpadDown",
  "dpadLeft",
  "dpadRight",
];

export type ControllerFamily = "xbox" | "playstation" | "nintendo" | "generic";
export type OutputKind = "auto" | "xbox360" | "ds4" | "keyboard" | "none";
/** What a stick turns into: itself, the mouse pointer, W/A/S/D or the arrow keys. */
export type StickMode = "stick" | "mouse" | "wasd" | "arrows";

/** A key or mouse button a controller button can be converted to (`name` is what a profile stores). */
export interface KeyInfo {
  name: string;
  group: "letters" | "digits" | "control" | "function" | "mouse";
}

/** A wireless controller's charge, when the system reports it. */
export interface ControllerBattery {
  percent: number;
  charging: boolean;
}

/** What a controller looks like, to show its picture. */
export type PadModel = "dualshock4" | "dualsense" | "steamdeck" | "other";

export interface ControllerInfo {
  id: string;
  name: string;
  path: string;
  vendor: number;
  product: number;
  family: ControllerFamily;
  model: PadModel;
  bus: "usb" | "bluetooth" | "other";
  battery?: ControllerBattery | null;
}

/** Physical state: sticks from -1 to 1 (Y grows downwards), triggers from 0 to 1. */
export interface PadSnapshot {
  pressed: ControllerButton[];
  lx: number;
  ly: number;
  rx: number;
  ry: number;
  lt: number;
  rt: number;
}

export interface ControllerProfile {
  output: OutputKind;
  /** Physical button → button the game receives; null disables it. */
  remap: Partial<Record<ControllerButton, ControllerButton | null>>;
  leftDeadzone: number;
  rightDeadzone: number;
  triggerDeadzone: number;
  rightCurve: number;
  invertLeftY: boolean;
  invertRightY: boolean;
  swapSticks: boolean;
  swapFace: boolean;
  rumble: boolean;
  /** Physical button → key or mouse button; it no longer reaches the gamepad. */
  keys: Partial<Record<ControllerButton, string>>;
  leftStick: StickMode;
  rightStick: StickMode;
  /** Pointer speed in pixels per second at full tilt, when a stick is the mouse. */
  mouseSpeed: number;
}

export interface ControllerStatus {
  controllers: ControllerInfo[];
  /** Whether Lutra may create virtual controllers. */
  uinput: boolean;
  /** Whether Lutra may create HID controllers, which PlayStation buttons in games need. */
  uhid: boolean;
  /** Who holds the controllers: a game id, or "test". */
  session: { owner: string; output: OutputKind } | null;
  navigation: boolean;
  /** Lutra drives the controllers system-wide with the default profile when no game runs. */
  system: boolean;
  /** Steam runs: outside games, the controllers are left to Steam Input. */
  steam: boolean;
}

export const TEST_SESSION = "test";

export const DEFAULT_PROFILE: ControllerProfile = {
  output: "auto",
  remap: {},
  leftDeadzone: 0.08,
  rightDeadzone: 0.08,
  triggerDeadzone: 0.04,
  rightCurve: 1,
  invertLeftY: false,
  invertRightY: false,
  swapSticks: false,
  swapFace: false,
  rumble: true,
  keys: {},
  leftStick: "stick",
  rightStick: "stick",
  mouseSpeed: 900,
};

// ---- browser preview ----

/** `?pad=dualsense` or `?pad=steamdeck` in the browser preview shows another controller. */
function mockModel(): PadModel {
  const asked = typeof location === "undefined" ? null : new URLSearchParams(location.search).get("pad");
  return asked === "dualsense" || asked === "steamdeck" ? asked : "dualshock4";
}

const MOCK_CONTROLLER: ControllerInfo = {
  id: "b7:a4:0b:54:ed:e9",
  name: "Wireless Controller",
  path: "/dev/input/event30",
  vendor: 0x054c,
  product: 0x09cc,
  family: "playstation",
  model: mockModel(),
  bus: "bluetooth",
  battery: { percent: 64, charging: false },
};
const mockProfiles = new Map<string, ControllerProfile>();
let mockSession: ControllerStatus["session"] = null;
let mockNavigation = true;
let mockSystem = true;

const MOCK_BUTTONS: ControllerButton[] = [
  "south", "east", "west", "north", "l1", "r1", "select", "start", "guide", "l3", "r3",
  "dpadUp", "dpadDown", "dpadLeft", "dpadRight",
];

/** A controller going through every button and moving its sticks, so the live view can be designed in a browser. */
function mockState(): PadSnapshot {
  const t = Date.now() / 1000;
  return {
    pressed: [MOCK_BUTTONS[Math.floor(t) % MOCK_BUTTONS.length]],
    lx: Math.cos(t) * 0.8,
    ly: Math.sin(t) * 0.8,
    rx: Math.sin(t * 0.7) * 0.9,
    ry: Math.cos(t * 0.7) * 0.9,
    lt: (Math.sin(t * 1.3) + 1) / 2,
    rt: (Math.cos(t * 1.1) + 1) / 2,
  };
}

// ---- commands ----

export function controllerStatus(): Promise<ControllerStatus> {
  if (hasBackend()) return invoke<ControllerStatus>("controller_status");
  return Promise.resolve({
    controllers: [MOCK_CONTROLLER],
    uinput: true,
    uhid: true,
    session: mockSession,
    navigation: mockNavigation,
    system: mockSystem,
    steam: false,
  });
}

export interface PadPreview {
  raw: PadSnapshot;
  /** What the game would receive with the profile. */
  output: PadSnapshot;
}

/** A controller before and after a profile, for the test screen. */
export function controllerPreview(id: string, profile: ControllerProfile): Promise<PadPreview | null> {
  if (hasBackend()) return invoke<PadPreview | null>("controller_preview", { id, profile });
  const raw = mockState();
  const dead = (x: number, y: number, zone: number) => {
    const m = Math.hypot(x, y);
    if (m <= zone) return [0, 0];
    const k = Math.min(1, (m - zone) / (1 - zone)) / m;
    return [x * k, y * k];
  };
  const [lx, ly] = dead(raw.lx, raw.ly, profile.leftDeadzone);
  const [rx, ry] = dead(raw.rx, raw.ry, profile.rightDeadzone);
  return Promise.resolve({ raw, output: { ...raw, lx, ly, rx, ry } });
}

const FACE_SWAP: Partial<Record<ControllerButton, ControllerButton>> = {
  south: "east",
  east: "south",
  north: "west",
  west: "north",
};

/** The key or mouse button a physical button is converted to, if any. */
export function keyFor(profile: ControllerProfile, button: ControllerButton): string | undefined {
  return profile.keys[profile.swapFace ? (FACE_SWAP[button] ?? button) : button];
}

/** What the game receives for a physical button (null: nothing), following the profile. */
export function mappedButton(profile: ControllerProfile, button: ControllerButton): ControllerButton | null {
  const physical = profile.swapFace ? (FACE_SWAP[button] ?? button) : button;
  if (profile.keys[physical]) return null;
  const mapped = profile.remap[physical];
  return mapped === undefined ? physical : mapped;
}

/** What a physical button becomes: nothing changed, nothing, another button, or a key / mouse button. */
export type Assignment =
  | { kind: "same" }
  | { kind: "off" }
  | { kind: "button"; button: ControllerButton }
  | { kind: "key"; key: string };

/** The profile's `remap` and `keys` once `button` is given `to`. Everything else is left alone. */
export function assignButton(
  profile: ControllerProfile,
  button: ControllerButton,
  to: Assignment,
): Pick<ControllerProfile, "remap" | "keys"> {
  const remap = { ...profile.remap };
  const keys = { ...profile.keys };
  delete remap[button];
  delete keys[button];
  if (to.kind === "key") keys[button] = to.key;
  else if (to.kind === "off") remap[button] = null;
  else if (to.kind === "button" && to.button !== button) remap[button] = to.button;
  return { remap, keys };
}

/** What a physical button is set to now. */
export function assignmentOf(profile: ControllerProfile, button: ControllerButton): Assignment {
  const key = profile.keys[button];
  if (key) return { kind: "key", key };
  const mapped = profile.remap[button];
  if (mapped === undefined) return { kind: "same" };
  return mapped === null ? { kind: "off" } : { kind: "button", button: mapped };
}

const MOCK_KEYS: KeyInfo[] = [
  ..."abcdefghijklmnopqrstuvwxyz".split("").map((name) => ({ name, group: "letters" as const })),
  ...["enter", "escape", "space", "tab", "up", "down", "left", "right"].map((name) => ({ name, group: "control" as const })),
  ...["mouse_left", "mouse_right", "wheel_up", "wheel_down"].map((name) => ({ name, group: "mouse" as const })),
];

let keyList: Promise<KeyInfo[]> | null = null;

/** The keys and mouse buttons a button can be converted to, as the backend knows them. */
export function controllerKeys(): Promise<KeyInfo[]> {
  keyList ??= hasBackend() ? invoke<KeyInfo[]>("controller_keys") : Promise.resolve(MOCK_KEYS);
  return keyList;
}

export const controllerState = (id: string): Promise<PadSnapshot | null> =>
  hasBackend() ? invoke<PadSnapshot | null>("controller_state", { id }) : Promise.resolve(mockState());

/** The default profile (no game) or the one a game plays with; `custom` says it is the game's own. */
export function controllerProfile(game: string | null): Promise<{ profile: ControllerProfile; custom: boolean }> {
  if (hasBackend()) return invoke("controller_profile", { game });
  const own = game ? mockProfiles.get(game) : undefined;
  return Promise.resolve({
    profile: own ?? mockProfiles.get("default") ?? DEFAULT_PROFILE,
    custom: own !== undefined,
  });
}

export function saveControllerProfile(game: string | null, profile: ControllerProfile): Promise<void> {
  if (hasBackend()) return invoke("save_controller_profile", { game, profile });
  mockProfiles.set(game ?? "default", profile);
  return Promise.resolve();
}

export function resetControllerProfile(game: string): Promise<void> {
  if (hasBackend()) return invoke("reset_controller_profile", { game });
  mockProfiles.delete(game);
  return Promise.resolve();
}

export function startControllerTest(profile: ControllerProfile): Promise<void> {
  if (hasBackend()) return invoke("start_controller_test", { profile });
  mockSession = { owner: TEST_SESSION, output: profile.output };
  return Promise.resolve();
}

export function stopControllerTest(): Promise<void> {
  if (hasBackend()) return invoke("stop_controller_test");
  mockSession = null;
  return Promise.resolve();
}

export function setControllerNavigation(enabled: boolean): Promise<void> {
  if (hasBackend()) return invoke("set_controller_navigation", { enabled });
  mockNavigation = enabled;
  return Promise.resolve();
}

export function setControllerSystem(enabled: boolean): Promise<void> {
  if (hasBackend()) return invoke("set_controller_system", { enabled });
  mockSystem = enabled;
  return Promise.resolve();
}

const noop: UnlistenFn = () => {};

export const onControllersChanged = (callback: () => void): Promise<UnlistenFn> =>
  hasBackend() ? listen("controllers-changed", () => callback()) : Promise.resolve(noop);

/** A controller's battery dropped to 30 % or 15 %. */
export const onControllerBattery = (
  callback: (alert: { id: string; name: string; percent: number; level: number }) => void,
): Promise<UnlistenFn> =>
  hasBackend()
    ? listen<{ id: string; name: string; percent: number; level: number }>("controller-battery", (event) => callback(event.payload))
    : Promise.resolve(noop);

/**
 * Menu actions sent by the controller driver while no game runs. In the browser preview,
 * `window.dispatchEvent(new CustomEvent("lutra:action", { detail: "powerMenu" }))` plays one.
 */
export const onControllerAction = (callback: (action: string) => void): Promise<UnlistenFn> => {
  if (hasBackend()) return listen<string>("ui-action", (event) => callback(event.payload));
  const onPreview = (event: Event) => callback(String((event as CustomEvent).detail));
  window.addEventListener("lutra:action", onPreview);
  return Promise.resolve(() => window.removeEventListener("lutra:action", onPreview));
};

// ---- power menu ----

/** The controller the power menu would switch off. `wired`: on a cable, which cannot be switched off. */
export interface PowerTarget {
  id: string;
  name: string;
  wired: boolean;
}

export const powerTarget = (): Promise<PowerTarget | null> =>
  hasBackend()
    ? invoke<PowerTarget | null>("controller_power_target")
    : Promise.resolve({ id: MOCK_CONTROLLER.id, name: MOCK_CONTROLLER.name, wired: false });

/** Ends the controller's Bluetooth link for real. Rejects with `wired` on a cable, or Bluetooth's own message. */
export const powerOffController = (id?: string): Promise<void> =>
  hasBackend() ? invoke<void>("power_off_controller", { id: id ?? null }) : Promise.resolve();
