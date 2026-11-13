/**
 * How systems are shown: libretro names them "Maker - Model" (its database names, which Lutra
 * uses as ids); people know them as "SNES" or "Mega Drive". Systems missing here still work,
 * with a name derived from the database one.
 */

export type PlatformKind = "home" | "handheld" | "arcade" | "computer";

export interface Platform {
  /** libretro database name. */
  id: string;
  /** What people call it, e.g. "SNES". */
  short: string;
  name: string;
  maker: string;
  year?: number;
  kind: PlatformKind;
  /** Accent of its tile. */
  color: string;
  /** Listed among the main systems (variants such as "DSi Decrypted" are not). */
  main: boolean;
}

type Entry = [id: string, short: string, name: string, year: number, kind: PlatformKind, color: string];

const CATALOG: Entry[] = [
  ["Nintendo - Nintendo Entertainment System", "NES", "Nintendo Entertainment System", 1983, "home", "#c8102e"],
  ["Nintendo - Family Computer Disk System", "FDS", "Famicom Disk System", 1986, "home", "#b5122b"],
  ["Nintendo - Super Nintendo Entertainment System", "SNES", "Super Nintendo", 1990, "home", "#6d4bc4"],
  ["Nintendo - Nintendo 64", "N64", "Nintendo 64", 1996, "home", "#1f8a3b"],
  ["Nintendo - GameCube", "GC", "GameCube", 2001, "home", "#5340a8"],
  ["Nintendo - Wii", "Wii", "Wii", 2006, "home", "#2c9fd9"],
  ["Nintendo - Game Boy", "GB", "Game Boy", 1989, "handheld", "#7b8b2f"],
  ["Nintendo - Game Boy Color", "GBC", "Game Boy Color", 1998, "handheld", "#8e3fbf"],
  ["Nintendo - Game Boy Advance", "GBA", "Game Boy Advance", 2001, "handheld", "#4a3fb5"],
  ["Nintendo - Nintendo DS", "DS", "Nintendo DS", 2004, "handheld", "#5b6472"],
  ["Nintendo - Virtual Boy", "VB", "Virtual Boy", 1995, "handheld", "#b3151c"],
  ["Sony - PlayStation", "PS1", "PlayStation", 1994, "home", "#4f5b6e"],
  ["Sony - PlayStation 2", "PS2", "PlayStation 2", 2000, "home", "#1d3f8f"],
  ["Sony - PlayStation Portable", "PSP", "PlayStation Portable", 2004, "handheld", "#2b2f3a"],
  ["Sega - SG-1000", "SG-1000", "SG-1000", 1983, "home", "#2d5fb3"],
  ["Sega - Master System - Mark III", "SMS", "Master System", 1985, "home", "#c4202f"],
  ["Sega - Mega Drive - Genesis", "MD", "Mega Drive", 1988, "home", "#1e1f24"],
  ["Sega - Mega-CD - Sega CD", "MCD", "Mega-CD", 1991, "home", "#2a2c36"],
  ["Sega - 32X", "32X", "32X", 1994, "home", "#b0262f"],
  ["Sega - Saturn", "SAT", "Saturn", 1994, "home", "#3d4252"],
  ["Sega - Dreamcast", "DC", "Dreamcast", 1998, "home", "#e2601f"],
  ["Sega - Game Gear", "GG", "Game Gear", 1990, "handheld", "#24262e"],
  ["Sega - PICO", "PICO", "Pico", 1993, "home", "#3aa0d8"],
  ["NEC - PC Engine - TurboGrafx 16", "PCE", "PC Engine", 1987, "home", "#e0742a"],
  ["NEC - PC Engine CD - TurboGrafx-CD", "PCE-CD", "PC Engine CD", 1988, "home", "#c9621f"],
  ["NEC - PC Engine SuperGrafx", "SGX", "SuperGrafx", 1989, "home", "#a8541d"],
  ["SNK - Neo Geo", "NEO GEO", "Neo Geo", 1990, "home", "#c49a2c"],
  ["SNK - Neo Geo Pocket", "NGP", "Neo Geo Pocket", 1998, "handheld", "#8c7a3a"],
  ["SNK - Neo Geo Pocket Color", "NGPC", "Neo Geo Pocket Color", 1999, "handheld", "#b08a2a"],
  ["Atari - 2600", "2600", "Atari 2600", 1977, "home", "#9a5a2b"],
  ["Atari - 7800", "7800", "Atari 7800", 1986, "home", "#6e4a2a"],
  ["Atari - Lynx", "LYNX", "Lynx", 1989, "handheld", "#51555f"],
  ["Bandai - WonderSwan", "WS", "WonderSwan", 1999, "handheld", "#7d8794"],
  ["Bandai - WonderSwan Color", "WSC", "WonderSwan Color", 2000, "handheld", "#3f7fbf"],
  ["The 3DO Company - 3DO", "3DO", "3DO", 1993, "home", "#3a3d45"],
  ["FBNeo - Arcade Games", "ARCADE", "Arcade (FinalBurn Neo)", 1978, "arcade", "#d63a4a"],
  ["MAME", "MAME", "Arcade (MAME)", 1978, "arcade", "#b82e5c"],
  ["Sega - Naomi", "NAOMI", "Naomi", 1998, "arcade", "#e07a24"],
  ["Atomiswave", "AW", "Atomiswave", 2003, "arcade", "#3a7bd5"],
  ["Commodore - 64", "C64", "Commodore 64", 1982, "computer", "#7a6a55"],
  ["Commodore - Amiga", "AMIGA", "Amiga", 1985, "computer", "#d04a3a"],
  ["Amstrad - CPC", "CPC", "Amstrad CPC", 1984, "computer", "#2e6db4"],
  ["ScummVM", "SCUMM", "ScummVM", 1987, "computer", "#2f9e57"],
];

/** Makers in the order the dashboard shows them; the others follow by name. */
export const MAKERS = ["Nintendo", "Sony", "Sega", "NEC", "SNK", "Atari", "Bandai", "Arcade", "Commodore", "Amstrad"];

/** libretro's maker names that are better shown otherwise. */
const MAKER_ALIASES: Record<string, string> = {
  FBNeo: "Arcade",
  MAME: "Arcade",
  Atomiswave: "Arcade",
  "The 3DO Company": "Panasonic",
  ScummVM: "PC",
};

const BY_ID = new Map(
  CATALOG.map(([id, short, name, year, kind, color]) => {
    const [maker] = id.split(" - ");
    const naomi = id === "Sega - Naomi";
    return [
      id,
      { id, short, name, year, kind, color, maker: naomi ? "Arcade" : (MAKER_ALIASES[maker] ?? maker), main: true },
    ];
  }),
);

/** Stable colour for systems missing from the catalogue. */
function hashColor(text: string): string {
  let hash = 0;
  for (const char of text) hash = (hash * 31 + char.charCodeAt(0)) | 0;
  return `hsl(${Math.abs(hash) % 360} 45% 38%)`;
}

export function platformOf(id: string): Platform {
  const known = BY_ID.get(id);
  if (known) return known;
  const [maker, ...rest] = id.split(" - ");
  const name = rest.length > 0 ? rest.join(" - ") : maker;
  return {
    id,
    short: name.length <= 8 ? name.toUpperCase() : name,
    name,
    // Empty: shown under "Other".
    maker: rest.length > 0 ? (MAKER_ALIASES[maker] ?? maker) : "",
    kind: "home",
    color: hashColor(id),
    main: false,
  };
}

/** Sorts makers in the dashboard's order. */
export function makerRank(maker: string): number {
  const index = MAKERS.indexOf(maker);
  return index === -1 ? MAKERS.length : index;
}
