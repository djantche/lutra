import type { RetroButton } from "./backend";

/**
 * The buttons of each console's controller, by what is printed on them, mapped to libretro's
 * pad (a Super Nintendo pad). The emulator, not Lutra, decides that mapping; these names
 * follow the emulators Lutra picks. The directional pad is always there and not listed.
 */
type Layout = [RetroButton, string][];

const SNES: Layout = [["b", "B"], ["a", "A"], ["y", "Y"], ["x", "X"], ["l", "L"], ["r", "R"], ["select", "Select"], ["start", "Start"]];
const NES: Layout = [["b", "B"], ["a", "A"], ["select", "Select"], ["start", "Start"]];
const GAME_BOY: Layout = NES;
const GBA: Layout = [["b", "B"], ["a", "A"], ["l", "L"], ["r", "R"], ["select", "Select"], ["start", "Start"]];
const DS: Layout = [...SNES];
const PLAYSTATION: Layout = [
  ["b", "✕"], ["a", "○"], ["y", "□"], ["x", "△"],
  ["l", "L1"], ["r", "R1"], ["l2", "L2"], ["r2", "R2"], ["l3", "L3"], ["r3", "R3"],
  ["select", "Select"], ["start", "Start"],
];
// Genesis Plus GX: the three main buttons, then the six-button pad's top row.
const MEGA_DRIVE: Layout = [["y", "A"], ["b", "B"], ["a", "C"], ["l", "X"], ["x", "Y"], ["r", "Z"], ["select", "Mode"], ["start", "Start"]];
const MASTER_SYSTEM: Layout = [["b", "1"], ["a", "2"], ["start", "Pause"]];
const PC_ENGINE: Layout = [["a", "I"], ["b", "II"], ["select", "Select"], ["start", "Run"]];
const NEO_GEO_POCKET: Layout = [["b", "A"], ["a", "B"], ["start", "Option"]];
const ARCADE: Layout = [["b", "1"], ["a", "2"], ["y", "3"], ["x", "4"], ["l", "5"], ["r", "6"], ["select", "Coin"], ["start", "Start"]];
const ATARI_2600: Layout = [["b", "Fire"], ["select", "Select"], ["start", "Reset"]];

const BY_SYSTEM: Record<string, Layout> = {
  "Nintendo - Nintendo Entertainment System": NES,
  "Nintendo - Family Computer Disk System": NES,
  "Nintendo - Super Nintendo Entertainment System": SNES,
  "Nintendo - Game Boy": GAME_BOY,
  "Nintendo - Game Boy Color": GAME_BOY,
  "Nintendo - Game Boy Advance": GBA,
  "Nintendo - Nintendo DS": DS,
  "Nintendo - Virtual Boy": GBA,
  "Sony - PlayStation": PLAYSTATION,
  "Sega - Mega Drive - Genesis": MEGA_DRIVE,
  "Sega - Mega-CD - Sega CD": MEGA_DRIVE,
  "Sega - 32X": MEGA_DRIVE,
  "Sega - Master System - Mark III": MASTER_SYSTEM,
  "Sega - Game Gear": MASTER_SYSTEM,
  "Sega - SG-1000": MASTER_SYSTEM,
  "NEC - PC Engine - TurboGrafx 16": PC_ENGINE,
  "NEC - PC Engine CD - TurboGrafx-CD": PC_ENGINE,
  "NEC - PC Engine SuperGrafx": PC_ENGINE,
  "SNK - Neo Geo Pocket": NEO_GEO_POCKET,
  "SNK - Neo Geo Pocket Color": NEO_GEO_POCKET,
  "FBNeo - Arcade Games": ARCADE,
  MAME: ARCADE,
  "SNK - Neo Geo": ARCADE,
  "Atari - 2600": ATARI_2600,
};

/** The buttons of a console, with their names; a full pad for consoles not listed. */
export function consoleButtons(system: string): Layout {
  return BY_SYSTEM[system] ?? PLAYSTATION.map(([button]) => [button, button.toUpperCase()] as [RetroButton, string]);
}

/** The controller button that presses a console button unless remapped: the same position. */
export const DEFAULT_BUTTON = {
  b: "south",
  a: "east",
  y: "west",
  x: "north",
  l: "l1",
  r: "r1",
  l2: "l2",
  r2: "r2",
  l3: "l3",
  r3: "r3",
  select: "select",
  start: "start",
  up: "dpadUp",
  down: "dpadDown",
  left: "dpadLeft",
  right: "dpadRight",
} as const satisfies Record<RetroButton, string>;
