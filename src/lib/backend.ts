import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { MOCK_GAMES } from "../data/mockGames";
import type { ControllerButton } from "./controllers";
import type { Game } from "../types/game";

/** True inside the Tauri window. In a plain browser (`pnpm dev`) the app runs on mock data. */
export const hasBackend = (): boolean => isTauri();

export const initialGames = (): Game[] => (hasBackend() ? [] : MOCK_GAMES);

export interface Runner {
  kind: "wine" | "proton";
  label: string;
  path: string;
}

/** How a game is run: through a compatibility layer, or directly (a Linux program). */
export type RunnerKind = Runner["kind"] | "native";

export interface ImageProgress {
  done: number;
  total: number;
  running: boolean;
}

/** Details of a game, from GameDB and editable by the user. */
export interface GameMeta {
  developer: string | null;
  /** `YYYY-MM-DD`, or just `YYYY`. */
  releaseDate: string | null;
  genres: string[];
  description: string | null;
  igdbId: number | null;
}

// ---- Emulation ----

export interface FirmwareStatus {
  description: string;
  path: string;
  optional: boolean;
  state: "present" | "different" | "missing";
}

export interface CoreView {
  id: string;
  displayName: string;
  version: string | null;
  /** `lutra` cores were downloaded and checked by Lutra; `system` ones come with the distribution. */
  origin: "lutra" | "system";
  /** libretro database names, one per system. */
  systems: string[];
  extensions: string[];
  firmware: FirmwareStatus[];
  /** Every required firmware is in place. */
  ready: boolean;
  hwRender: boolean;
}

export interface EmulationStatus {
  retroarch: { path: string; version: string | null } | null;
  cores: CoreView[];
  systems: { name: string; cores: string[] }[];
  romDirs: string[];
  biosDir: string;
  defaultBiosDir: string;
  /** The BIOS folder of the user's own RetroArch, when it is not the one in use. */
  suggestedBiosDir: string | null;
}

export interface ScanSummary {
  added: number;
  removed: number;
  total: number;
  /** Disc images in folders that do not say which system they are for. */
  unplaced: number;
  /** Games whose system has no installed core. */
  withoutCore: number;
}

/** A console button, by libretro's names (a Super Nintendo pad: B at the bottom, A on the right). */
export type RetroButton =
  | "b" | "y" | "select" | "start" | "up" | "down" | "left" | "right"
  | "a" | "x" | "l" | "r" | "l2" | "r2" | "l3" | "r3";

export interface EmulationProfile {
  video: {
    /** Sharp square pixels, smoothed, or whole multiples only. */
    scaling: "sharp" | "smooth" | "integer";
    aspect: "console" | "square" | "stretch";
    effect: "none" | "scanlines" | "lcd";
  };
  controls: {
    /** The controller button that presses each console button, when not the same position. */
    remap: Partial<Record<RetroButton, ControllerButton>>;
    /** The left stick also works as the directional pad. */
    stickAsDpad: boolean;
  };
  /** The emulator's options the user changed; the others keep the core's defaults. */
  coreOptions: Record<string, string>;
}

export const DEFAULT_EMULATION_PROFILE: EmulationProfile = {
  video: { scaling: "sharp", aspect: "console", effect: "none" },
  controls: { remap: {}, stickAsDpad: false },
  coreOptions: {},
};

export interface CoreOption {
  key: string;
  label: string;
  info: string | null;
  category: string | null;
  values: { value: string; label: string }[];
  default: string;
  visible: boolean;
}

export interface CoreOptions {
  categories: { key: string; label: string; info: string | null }[];
  options: CoreOption[];
}

export interface EmulatorConfig {
  system: string;
  fullscreen: boolean;
  autoResume: boolean;
  /** Settings kept for this game only; its console's otherwise. */
  profile: EmulationProfile | null;
}

export interface EmulatorSettings {
  id: string;
  title: string;
  romPath: string;
  config: EmulatorConfig;
}

export interface CoreChoice {
  id: string;
  name: string;
  origin: CoreView["origin"];
  ready: boolean;
}

export interface EmulatorSettingsView {
  settings: EmulatorSettings;
  /** What the game uses when it has no settings of its own. */
  consoleProfile: EmulationProfile;
  /** Installed cores that can play this game, the preferred one first. */
  cores: CoreChoice[];
}

/** A game found in GameDB, with the details it would fill in. */
export interface GameHit {
  id: number;
  name: string;
  year: number | null;
  developer: string | null;
  /** Cover image URL, for the preview. */
  cover: string | null;
  /** Released on PC: the likeliest match for a game added here. */
  pc: boolean;
  meta: GameMeta;
}

/** A program that appeared in the prefix while an installer ran. */
export interface InstallCandidate {
  path: string;
  /** Path under the prefix's C: drive, for display. */
  shown: string;
  size: number;
  score: number;
}

export interface InstallRequest {
  installerPath: string;
  title: string;
  runnerKind: Runner["kind"];
  runnerPath: string;
  /** An existing prefix; a new one is created when absent. */
  prefixPath?: string;
}

export interface InstallFinished {
  installId: string;
  prefixPath: string;
  success: boolean;
  message: string | null;
  candidates: InstallCandidate[];
}

export interface NewGame {
  title: string;
  exePath: string;
  runnerKind: RunnerKind;
  runnerPath: string;
  /** An existing Wine prefix to reuse; a new one is created when absent. */
  prefixPath?: string;
  meta?: GameMeta;
  /** The options to start with; the backend picks the best ones for this machine when absent. */
  config?: WineConfig;
}

export interface EnvVar {
  key: string;
  value: string;
}

/** Per-game Wine/Proton options. */
export interface WineConfig {
  args: string;
  workingDir: string | null;
  env: EnvVar[];
  dllOverrides: string;
  esync: boolean;
  fsync: boolean;
  wined3d: boolean;
  virtualDesktop: string | null;
  arch32: boolean;
  gamemode: boolean;
  mangohud: boolean;
  debug: boolean;
  /** PCI address of the GPU to render on; null leaves the system's choice. */
  gpu: string | null;
  fpsLimit: number | null;
  dxvkHud: string | null;
  ntsync: boolean;
  nvapi: boolean;
  wayland: boolean;
  fsr: boolean;
  perfTweaks: boolean;
  gamescope: Gamescope | null;
}

export interface Gamescope {
  width: number;
  height: number;
  fullscreen: boolean;
  fsr: boolean;
}

export interface Gpu {
  id: string;
  vendor: "intel" | "amd" | "nvidia" | "other";
  name: string;
  primary: boolean;
  driver: string | null;
}

export interface Hardware {
  gpus: Gpu[];
  ntsync: boolean;
  gamescope: string | null;
}

/** A Wine prefix found on this machine. */
export interface PrefixInfo {
  path: string;
  name: string;
  origin: "lutra" | "steam";
  /** Null for a folder where nothing ran yet. */
  kind: Runner["kind"] | null;
  arch: "win64" | "win32" | null;
}

/** One real stage of a game launch, pushed by the backend. */
export interface LaunchProgress {
  id: string;
  stage: "check" | "core" | "start" | "prefix" | "wait";
  step: number;
  steps: number;
  state: "progress" | "ready" | "failed" | "ended";
  message?: string;
}

export interface WineSettings {
  id: string;
  title: string;
  exePath: string;
  runnerKind: RunnerKind;
  runnerPath: string;
  prefixPath: string;
  config: WineConfig;
  /** Whether the prefix already exists on disk (it is created on first launch). */
  prefixExists: boolean;
}

/** One image offered for a game. `style` is "official" for Steam's own artwork. */
export interface Candidate {
  url: string;
  thumb: string;
  width: number | null;
  height: number | null;
  author: string | null;
  style: string | null;
}

export interface ArtworkChoices {
  posters: Candidate[];
  heroes: Candidate[];
}

export interface LogContent {
  path: string;
  text: string;
  /** Older lines were left out. */
  truncated: boolean;
}

/** Helper programs found on this machine; null when missing. */
export interface Tools {
  wine: string | null;
  wineVersion: string | null;
  umu: string | null;
  winetricks: string | null;
  gamemode: string | null;
  mangohud: string | null;
}

export interface SearchHit {
  id: number;
  name: string;
  year: number | null;
  verified: boolean;
}

/** Raw bytes of a downloaded artwork file. */
export const readArtwork = (path: string) => invoke<ArrayBuffer>("read_artwork", { path });

export const listGames = () => invoke<Game[]>("list_games");
export const scanSources = () => invoke<Game[]>("scan_sources");
export const setFavorite = (id: string, favorite: boolean) => invoke<void>("set_favorite", { id, favorite });
export const launchGameBackend = (id: string) => invoke<void>("launch_game", { id });

// ---- Windows games ----

const MOCK_RUNNERS: Runner[] = [
  { kind: "proton", label: "GE-Proton9-27", path: "/mock/GE-Proton9-27/proton" },
  { kind: "wine", label: "Wine", path: "/usr/bin/wine" },
];

export const listRunners = (): Promise<Runner[]> =>
  hasBackend() ? invoke<Runner[]>("list_runners") : Promise.resolve(MOCK_RUNNERS);

/** Opens the native file picker. Resolves to null if cancelled. */
export async function pickProgram(): Promise<string | null> {
  if (!hasBackend()) return "/mock/Games/Demo Game/demo.exe";
  const picked = await openDialog({
    multiple: false,
    directory: false,
    filters: [{ name: "Windows", extensions: ["exe", "bat"] }],
  });
  return typeof picked === "string" ? picked : null;
}

/** Opens the native file picker for a Linux program (binary, AppImage or script): no filter. */
export async function pickLinuxProgram(): Promise<string | null> {
  if (!hasBackend()) return "/mock/Games/Linux Demo/run.sh";
  const picked = await openDialog({ multiple: false, directory: false });
  return typeof picked === "string" ? picked : null;
}

/** Opens the native folder picker. Resolves to null if cancelled. */
export async function pickFolder(): Promise<string | null> {
  if (!hasBackend()) return "/mock/Prefixes/demo";
  const picked = await openDialog({ multiple: false, directory: true });
  return typeof picked === "string" ? picked : null;
}

const MOCK_TOOLS: Tools = {
  wine: "/usr/bin/wine",
  wineVersion: "wine-10.0",
  umu: "/usr/bin/umu-run",
  winetricks: null,
  gamemode: "/usr/bin/gamemoderun",
  mangohud: null,
};
const MOCK_CONFIG: WineConfig = {
  args: "",
  workingDir: null,
  env: [],
  dllOverrides: "",
  esync: true,
  fsync: true,
  wined3d: false,
  virtualDesktop: null,
  arch32: false,
  gamemode: false,
  mangohud: false,
  debug: false,
  gpu: null,
  fpsLimit: null,
  dxvkHud: null,
  ntsync: false,
  nvapi: false,
  wayland: false,
  fsr: false,
  perfTweaks: true,
  gamescope: null,
};
const MOCK_HARDWARE: Hardware = {
  gpus: [
    { id: "0000:00:02.0", vendor: "intel", name: "Intel UHD Graphics 630", primary: true, driver: "i915" },
    { id: "0000:01:00.0", vendor: "nvidia", name: "NVIDIA GeForce GTX 1060 Mobile", primary: false, driver: "nvidia" },
  ],
  ntsync: true,
  gamescope: null,
};
const MOCK_PREFIXES: PrefixInfo[] = [
  { path: "/mock/Prefixes/shared", name: "shared", origin: "lutra", kind: "proton", arch: "win64" },
  { path: "/mock/Steam/compatdata/730", name: "Counter-Strike 2", origin: "steam", kind: "proton", arch: "win64" },
];

export const wineTools = (): Promise<Tools> => (hasBackend() ? invoke<Tools>("wine_tools") : Promise.resolve(MOCK_TOOLS));

export const defaultRunner = (): Promise<string | null> =>
  hasBackend() ? invoke<string | null>("default_runner") : Promise.resolve(null);

export const setDefaultRunner = (path: string | null): Promise<void> =>
  hasBackend() ? invoke<void>("set_default_runner", { path }) : Promise.resolve();

export function wineSettings(id: string): Promise<WineSettings> {
  if (hasBackend()) return invoke<WineSettings>("wine_settings", { id });
  return Promise.resolve({
    id,
    title: "Demo Game",
    exePath: "/mock/Games/Demo Game/demo.exe",
    runnerKind: "proton",
    runnerPath: MOCK_RUNNERS[0].path,
    prefixPath: "/mock/Prefixes/demo",
    config: MOCK_CONFIG,
    prefixExists: false,
  });
}

export function saveWineSettings(settings: WineSettings): Promise<Game> {
  if (hasBackend()) return invoke<Game>("save_wine_settings", { settings });
  const known = MOCK_GAMES.find((game) => game.id === settings.id) ?? MOCK_GAMES[0];
  return Promise.resolve({ ...known, id: settings.id, title: settings.title });
}

export const stopGameBackend = (id: string) => (hasBackend() ? invoke<void>("stop_game", { id }) : Promise.resolve());
export const runningGames = (): Promise<string[]> =>
  hasBackend() ? invoke<string[]>("running_games") : Promise.resolve([]);
export const launchWineTool = (id: string, tool: "winecfg" | "winetricks") =>
  hasBackend() ? invoke<void>("launch_wine_tool", { id, tool }) : Promise.resolve();
export const runInPrefix = (id: string, exePath: string) =>
  hasBackend() ? invoke<void>("run_in_prefix", { id, exePath }) : Promise.resolve();
export const openGameFolder = (id: string, what: "files" | "prefix" | "logs") =>
  hasBackend() ? invoke<void>("open_game_folder", { id, what }) : Promise.resolve();
let mockCompat = false;
export const renderCompat = (): Promise<boolean> =>
  hasBackend() ? invoke<boolean>("render_compat") : Promise.resolve(mockCompat);
export function setRenderCompat(enabled: boolean): Promise<void> {
  if (hasBackend()) return invoke<void>("set_render_compat", { enabled });
  mockCompat = enabled;
  return Promise.resolve();
}

export const openPrefixesFolder = () => (hasBackend() ? invoke<void>("open_prefixes_folder") : Promise.resolve());

export function addGame(game: NewGame): Promise<Game> {
  if (hasBackend()) return invoke<Game>("add_game", { game });
  return Promise.resolve({
    id: `${game.runnerKind === "native" ? "native" : "wine"}:${Math.abs(game.exePath.length * 7919)}`,
    title: game.title || "Game",
    source: game.runnerKind === "native" ? "native" : "wine",
    hue: (game.exePath.length * 37) % 360,
    genres: game.meta?.genres ?? [],
    developer: game.meta?.developer ?? undefined,
    description: game.meta?.description ?? undefined,
    playtimeMinutes: 0,
    lastPlayed: null,
    favorite: false,
    exePath: game.exePath,
    runnerKind: game.runnerKind === "native" ? undefined : game.runnerKind,
  });
}

const MOCK_HITS: GameHit[] = [
  {
    id: 96209,
    name: "FIFA 19",
    year: 2018,
    developer: "EA Canada",
    cover: null,
    pc: true,
    meta: {
      developer: "EA Canada",
      releaseDate: "2018-09-26",
      genres: ["Sport"],
      description: "Football, with the UEFA Champions League at its heart.",
      igdbId: 96209,
    },
  },
  {
    id: 240453,
    name: "FIFA 19",
    year: 2018,
    developer: "EA Vancouver",
    cover: null,
    pc: false,
    meta: { developer: "EA Vancouver", releaseDate: "2018-09-28", genres: ["Sport"], description: null, igdbId: 240453 },
  },
];

/** Looks a game up in GameDB. */
export const searchGameDb = (query: string): Promise<GameHit[]> =>
  hasBackend()
    ? invoke<GameHit[]>("search_game_db", { query })
    : Promise.resolve(query.trim().length >= 2 ? MOCK_HITS : []);

/** Saves a game's details as the user edited them. */
export function saveGameMeta(id: string, meta: GameMeta): Promise<Game> {
  if (hasBackend()) return invoke<Game>("save_game_meta", { id, meta });
  const known = MOCK_GAMES.find((game) => game.id === id) ?? MOCK_GAMES[0];
  return Promise.resolve({
    ...known,
    developer: meta.developer ?? undefined,
    description: meta.description ?? undefined,
    genres: meta.genres,
    releaseDate: meta.releaseDate ?? undefined,
  });
}

/** Runs a setup program in a prefix. Resolves once it has started; `onInstallFinished` tells when it closed. */
export const startInstall = (request: InstallRequest): Promise<{ installId: string; prefixPath: string }> =>
  hasBackend()
    ? invoke<{ installId: string; prefixPath: string }>("start_install", { request })
    : Promise.resolve({ installId: "install:mock", prefixPath: request.prefixPath ?? "/mock/Prefixes/new" });

export const cancelInstall = (installId: string): Promise<void> =>
  hasBackend() ? invoke<void>("cancel_install", { installId }) : Promise.resolve();

const MOCK_CORES: CoreView[] = [
  {
    id: "snes9x",
    displayName: "Nintendo - SNES / SFC (Snes9x)",
    version: "1.63",
    origin: "system",
    systems: ["Nintendo - Super Nintendo Entertainment System"],
    extensions: ["smc", "sfc"],
    firmware: [],
    ready: true,
    hwRender: false,
  },
  {
    id: "swanstation",
    displayName: "Sony - PlayStation (SwanStation)",
    version: "0.1",
    origin: "lutra",
    systems: ["Sony - PlayStation"],
    extensions: ["cue", "chd", "iso"],
    firmware: [{ description: "scph5501.bin (PS1 US BIOS)", path: "scph5501.bin", optional: false, state: "missing" }],
    ready: false,
    hwRender: true,
  },
];
let mockEmuDirs = { romDirs: ["/mock/Roms"], biosDir: "/mock/Bios" };

export const emulationStatus = (): Promise<EmulationStatus> =>
  hasBackend()
    ? invoke<EmulationStatus>("emulation_status")
    : Promise.resolve({
        retroarch: { path: "/usr/bin/retroarch", version: "1.22.2" },
        cores: MOCK_CORES,
        systems: [],
        ...mockEmuDirs,
        defaultBiosDir: "/mock/lutra/emulation/system",
        suggestedBiosDir: "/mock/Games/bios",
      });

export function setEmulationDirs(romDirs: string[], biosDir: string | null): Promise<void> {
  if (hasBackend()) return invoke<void>("set_emulation_dirs", { romDirs, biosDir });
  mockEmuDirs = { romDirs, biosDir: biosDir ?? "/mock/lutra/emulation/system" };
  return Promise.resolve();
}

export const scanRoms = (): Promise<ScanSummary> =>
  hasBackend()
    ? invoke<ScanSummary>("scan_roms")
    : Promise.resolve({ added: 3, removed: 0, total: 3, unplaced: 1, withoutCore: 1 });

export const installCore = (coreId: string): Promise<{ name: string; version: string }> =>
  hasBackend() ? invoke<{ name: string; version: string }>("install_core", { coreId }) : Promise.resolve({ name: coreId, version: "1.0" });

export const emulatorSettings = (id: string): Promise<EmulatorSettingsView> =>
  hasBackend()
    ? invoke<EmulatorSettingsView>("emulator_settings", { id })
    : Promise.resolve({
        settings: {
          id,
          title: "Demo ROM",
          romPath: "/mock/Roms/demo.sfc",
          config: { system: "Nintendo - Super Nintendo Entertainment System", fullscreen: true, autoResume: false, profile: null },
        },
        consoleProfile: DEFAULT_EMULATION_PROFILE,
        cores: [{ id: "snes9x", name: "Nintendo - SNES / SFC (Snes9x)", origin: "system", ready: true }],
      });

export function saveEmulatorSettings(settings: EmulatorSettings): Promise<Game> {
  if (hasBackend()) return invoke<Game>("save_emulator_settings", { settings });
  const known = MOCK_GAMES.find((game) => game.id === settings.id) ?? MOCK_GAMES[0];
  return Promise.resolve({ ...known, id: settings.id, title: settings.title });
}

export interface SystemView {
  /** libretro database name, e.g. "Nintendo - Super Nintendo Entertainment System". */
  name: string;
  /** Installed cores for it, in the order Lutra tries them: the first is the one used. */
  cores: CoreChoice[];
  /** The core Lutra installs when none is. */
  recommendedCore: string | null;
  /** BIOS files of the core in use. */
  firmware: FirmwareStatus[];
  games: number;
  /** File extensions the installed cores accept. */
  extensions: string[];
  /** Picture of the console, from RetroArch's assets (read through `useArtworkUrl`). */
  icon: string | null;
  /** Its emulator draws with the GPU: it still runs in RetroArch, not in Lutra's player. */
  needsRetroarch: boolean;
}

const MOCK_SYSTEMS: SystemView[] = [
  {
    name: "Nintendo - Super Nintendo Entertainment System",
    cores: [{ id: "snes9x", name: "Nintendo - SNES / SFC (Snes9x)", origin: "system", ready: true }],
    recommendedCore: "snes9x",
    firmware: [],
    games: 2,
    extensions: ["sfc", "smc", "zip"],
    icon: null,
    needsRetroarch: false,
  },
  {
    name: "Sony - PlayStation",
    cores: [{ id: "swanstation", name: "Sony - PlayStation (SwanStation)", origin: "lutra", ready: false }],
    recommendedCore: "swanstation",
    firmware: [{ description: "PS1 US BIOS", path: "scph5501.bin", optional: false, state: "missing" }],
    games: 1,
    extensions: ["cue", "chd", "pbp"],
    icon: null,
    needsRetroarch: false,
  },
  {
    name: "Nintendo - Game Boy Advance",
    cores: [],
    recommendedCore: "mgba",
    firmware: [],
    games: 0,
    extensions: [],
    icon: null,
    needsRetroarch: false,
  },
  {
    name: "Sega - Mega Drive - Genesis",
    cores: [{ id: "genesis_plus_gx", name: "Sega - MS/GG/MD/CD (Genesis Plus GX)", origin: "system", ready: true }],
    recommendedCore: "genesis_plus_gx",
    firmware: [],
    games: 0,
    extensions: ["md", "gen", "bin", "zip"],
    icon: null,
    needsRetroarch: false,
  },
];

/** Every system worth showing on the emulation dashboard. */
export const emulationSystems = (): Promise<SystemView[]> =>
  hasBackend() ? invoke<SystemView[]>("emulation_systems") : Promise.resolve(MOCK_SYSTEMS);

export interface NewRom {
  path: string;
  system: string;
  title: string;
  meta: GameMeta | null;
}

export function addRom(rom: NewRom): Promise<Game> {
  if (hasBackend()) return invoke<Game>("add_rom", { rom });
  return Promise.resolve({
    id: `emulator:${Math.abs(rom.path.length * 7919)}`,
    title: rom.title || "ROM",
    source: "emulator",
    system: rom.system,
    hue: (rom.path.length * 37) % 360,
    genres: rom.meta?.genres ?? [],
    playtimeMinutes: 0,
    lastPlayed: null,
    favorite: false,
  });
}

/** Opens the native file picker for a game of a system; no filter when its formats are unknown. */
export async function pickRom(extensions: string[]): Promise<string | null> {
  if (!hasBackend()) return "/mock/Roms/Super Metroid (USA).sfc";
  const picked = await openDialog({
    multiple: false,
    directory: false,
    filters: extensions.length > 0 ? [{ name: "ROM", extensions }] : undefined,
  });
  return typeof picked === "string" ? picked : null;
}

const mockProfiles = new Map<string, EmulationProfile>();

/** The settings a console's games share. */
export const emulationProfile = (system: string): Promise<EmulationProfile> =>
  hasBackend()
    ? invoke<EmulationProfile>("emulation_profile", { system })
    : Promise.resolve(mockProfiles.get(system) ?? DEFAULT_EMULATION_PROFILE);

export function saveEmulationProfile(system: string, profile: EmulationProfile): Promise<void> {
  if (hasBackend()) return invoke<void>("save_emulation_profile", { system, profile });
  mockProfiles.set(system, profile);
  return Promise.resolve();
}

const MOCK_CORE_OPTIONS: CoreOptions = {
  categories: [
    { key: "video", label: "Vidéo", info: null },
    { key: "system", label: "Système", info: null },
  ],
  options: [
    { key: "snes9x_hires_blend", label: "Fusion des pixels haute résolution", info: "Adoucit les jeux qui alternent deux résolutions.", category: "video", values: [{ value: "disabled", label: "disabled" }, { value: "merge", label: "Fusionner" }, { value: "blur", label: "Flou" }], default: "disabled", visible: true },
    { key: "snes9x_overclock", label: "Accélération du processeur", info: "Réduit les ralentissements de certains jeux.", category: "system", values: [{ value: "disabled", label: "disabled" }, { value: "enabled", label: "enabled" }], default: "disabled", visible: true },
  ],
};

/** The options the emulator of a console offers, as it describes them. */
export const coreOptions = (system: string): Promise<CoreOptions> =>
  hasBackend() ? invoke<CoreOptions>("core_options", { system }) : Promise.resolve(MOCK_CORE_OPTIONS);

/** The interface's language, which the backend hands to emulators. */
export const setUiLanguage = (language: string): Promise<void> =>
  hasBackend() ? invoke<void>("set_ui_language", { language }) : Promise.resolve();

export const openEmulationFolder = (what: "bios" | "saves" | "states" | "cores") =>
  hasBackend() ? invoke<void>("open_emulation_folder", { what }) : Promise.resolve();

/** Closes the in-game overlay, optionally bringing the Lutra window to the front. */
export const closeOverlay = (showLutra: boolean): Promise<void> =>
  hasBackend() ? invoke<void>("close_overlay", { showLutra }) : Promise.resolve();

export const hardwareInfo = (): Promise<Hardware> =>
  hasBackend() ? invoke<Hardware>("hardware_info") : Promise.resolve(MOCK_HARDWARE);

/** The options that suit this machine best for a kind of runner. */
export const recommendedConfig = (runnerKind: RunnerKind): Promise<WineConfig> =>
  hasBackend()
    ? invoke<WineConfig>("recommended_config", { runnerKind })
    : Promise.resolve({ ...MOCK_CONFIG, gpu: MOCK_HARDWARE.gpus[1].id, gamemode: true });

export const listPrefixes = (): Promise<PrefixInfo[]> =>
  hasBackend() ? invoke<PrefixInfo[]>("list_prefixes") : Promise.resolve(MOCK_PREFIXES);

/** Resolves when a folder can be the prefix of a game run by `runnerKind`; rejects with the reason. */
export const checkPrefix = (path: string, runnerKind: RunnerKind): Promise<void> =>
  hasBackend() ? invoke<void>("check_prefix", { path, runnerKind }) : Promise.resolve();

/** `everything`: the Wine prefix, logs and controller profile go too. */
export const removeGameBackend = (id: string, everything: boolean) =>
  hasBackend() ? invoke<void>("remove_game", { id, everything }) : Promise.resolve();

/** The prefix removing a game can delete too: one of Lutra's own, used by no other game. */
export const removablePrefix = (id: string): Promise<{ path: string; bytes: number } | null> =>
  hasBackend()
    ? invoke<{ path: string; bytes: number } | null>("removable_prefix_of", { id })
    : Promise.resolve({ path: `/mock/lutra/prefixes/${id}`, bytes: 2_400_000_000 });

// ---- SteamGridDB ----

export const STEAMGRIDDB_KEY_URL = "https://www.steamgriddb.com/profile/preferences/api";

export interface ArtSettings {
  /** Where Steam games get their images from. */
  steamSource: "official" | "custom";
  /** With custom images, use Steam's own when SteamGridDB has none. */
  fallback: boolean;
  hasKey: boolean;
}

const MOCK_ART: ArtSettings = { steamSource: "official", fallback: true, hasKey: false };
let mockArt = MOCK_ART;

export const artSettings = (): Promise<ArtSettings> =>
  hasBackend() ? invoke<ArtSettings>("art_settings") : Promise.resolve(mockArt);

export function setArtSettings(steamSource: ArtSettings["steamSource"], fallback: boolean): Promise<ArtSettings> {
  if (hasBackend()) return invoke<ArtSettings>("set_art_settings", { steamSource, fallback });
  mockArt = { ...mockArt, steamSource, fallback };
  return Promise.resolve(mockArt);
}

/** Resolves to true once a key is saved, false once removed. Rejects with "invalid_key" if refused. */
export function saveSteamGridDbKey(key: string): Promise<boolean> {
  if (hasBackend()) return invoke<boolean>("save_steamgriddb_key", { key });
  if (key.trim() === "invalid") return Promise.reject("invalid_key");
  mockArt = { ...mockArt, hasKey: key.trim() !== "" };
  return Promise.resolve(mockArt.hasKey);
}

/** Rejects with "no_key" when no SteamGridDB key is saved. */
export function searchArtwork(query: string): Promise<SearchHit[]> {
  if (hasBackend()) return invoke<SearchHit[]>("search_artwork", { query });
  if (!mockArt.hasKey) return Promise.reject("no_key");
  return Promise.resolve(
    ["", " (Remastered)", " 2"].map((suffix, index) => ({
      id: index + 1,
      name: query + suffix,
      year: 2015 + index * 4,
      verified: index === 0,
    })),
  );
}

/** Uses a SteamGridDB game for a game's artwork; `null` goes back to automatic. */
export const pickArtwork = (id: string, sgdbGame: number | null) =>
  hasBackend() ? invoke<void>("pick_artwork", { id, sgdbGame }) : Promise.resolve();

/** The SteamGridDB game used for a game's artwork. Rejects with "no_key" without a key. */
export function artworkMatch(id: string, title: string): Promise<SearchHit | null> {
  if (hasBackend()) return invoke<SearchHit | null>("artwork_match", { id });
  if (!mockArt.hasKey) return Promise.reject("no_key");
  return Promise.resolve({ id: 1, name: title, year: 2019, verified: true });
}

/** Stand-in images for the browser preview: coloured panels instead of real artwork. */
function mockCandidate(hue: number, width: number, height: number, style: string | null): Candidate {
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 ${width} ${height}'><defs><linearGradient id='g' x1='0' y1='0' x2='1' y2='1'><stop offset='0' stop-color='hsl(${hue} 70% 55%)'/><stop offset='1' stop-color='hsl(${hue + 40} 70% 25%)'/></linearGradient></defs><rect width='100%' height='100%' fill='url(%23g)'/></svg>`;
  const url = `data:image/svg+xml,${svg.replace(/#/g, "%23")}`;
  return { url, thumb: url, width, height, author: style === "official" ? null : "Someone", style };
}

export function artworkChoices(id: string, sgdbGame: number | null): Promise<ArtworkChoices> {
  if (hasBackend()) return invoke<ArtworkChoices>("artwork_choices", { id, sgdbGame });
  const hues = sgdbGame === null ? [] : [20, 80, 140, 200, 260, 320];
  // Like the backend: Steam's own artwork is only offered for Steam games.
  const steam = id.startsWith("steam:");
  return Promise.resolve({
    posters: [
      ...(steam ? [mockCandidate(210, 600, 900, "official")] : []),
      ...hues.map((hue) => mockCandidate(hue, 600, 900, "alternate")),
    ],
    heroes: [
      ...(steam ? [mockCandidate(210, 1920, 620, "official")] : []),
      ...hues.slice(0, 3).map((hue) => mockCandidate(hue, 1920, 620, null)),
    ],
  });
}

/** Downloads a chosen image as a game's cover (`poster`) or banner (`hero`). */
export function setArtwork(id: string, kind: "poster" | "hero", url: string): Promise<Game> {
  if (hasBackend()) return invoke<Game>("set_artwork", { id, kind, url });
  const known = MOCK_GAMES.find((game) => game.id === id) ?? MOCK_GAMES[0];
  return Promise.resolve({ ...known, [kind === "poster" ? "posterUrl" : "heroUrl"]: url });
}

/** The logs a game can have: this run's output, the run before, and the runner's and graphics layers' (debug mode). */
export type GameLogKind = "game" | "previous" | "proton" | "graphics";

export const gameLogs = (id: string): Promise<GameLogKind[]> =>
  hasBackend() ? invoke<GameLogKind[]>("game_logs", { id }) : Promise.resolve(["game", "previous"]);

export function readLog(target: string, kind: GameLogKind = "game"): Promise<LogContent> {
  if (hasBackend()) return invoke<LogContent>("read_log", { target, kind });
  const now = new Date().toISOString().slice(0, 19).replace("T", " ");
  return Promise.resolve({
    path: `/mock/logs/${target}.log`,
    truncated: false,
    text:
      target === "app"
        ? `${now} INFO  Lutra 0.1.0 started\n${now} INFO  Scan: 4 Steam game(s) installed\n${now} WARN  No artwork for Demo: timeout\n`
        : `=== Lutra · ${now} ===\nCommand: umu-run /games/demo.exe\nFolder: /games\nEnv: WINEPREFIX=/prefixes/demo\n===\n\n0024:err:module:import_dll Library d3dx9_43.dll not found\nfixme:ntdll:NtQuerySystemInformation info_class SYSTEM_PERFORMANCE_INFORMATION\n`,
  });
}

export const logFrontend = (level: "info" | "warn" | "error", message: string) =>
  hasBackend() ? invoke<void>("log_frontend", { level, message }) : Promise.resolve();

export const openLogsFolder = () => (hasBackend() ? invoke<void>("open_logs_folder") : Promise.resolve());

export const importImages = (force: boolean): Promise<void> =>
  hasBackend() ? invoke<void>("import_images", { force }) : Promise.resolve();

export const openExternal = (url: string): Promise<void> =>
  hasBackend() ? openUrl(url) : Promise.resolve(void window.open(url, "_blank", "noopener"));

// ---- events pushed by the backend ----

const noop: UnlistenFn = () => {};

// ---- Notification area ----

export interface TraySettings {
  /** Closing the window keeps Lutra running in the notification area. */
  closeToTray: boolean;
  /** Lutra starts in the notification area, its window hidden. */
  startInTray: boolean;
  /** The window hides while a game runs and comes back when it ends. */
  hideDuringGames: boolean;
  /** A notification area exists on this desktop: without one, nothing is ever hidden. */
  available: boolean;
}

let mockTray: TraySettings = { closeToTray: true, startInTray: false, hideDuringGames: true, available: true };

export const traySettings = (): Promise<TraySettings> =>
  hasBackend() ? invoke<TraySettings>("tray_settings") : Promise.resolve(mockTray);

export function setTraySettings(settings: TraySettings): Promise<void> {
  if (hasBackend()) return invoke<void>("set_tray_settings", { settings });
  mockTray = settings;
  return Promise.resolve();
}

/** The tray menu's words, in the interface's language. */
export const setTrayLabels = (labels: { open: string; bigPicture: string; quit: string }): Promise<void> =>
  hasBackend() ? invoke<void>("set_tray_labels", { labels }) : Promise.resolve();

/** "Big Picture" was picked in the tray menu. */
export const onTrayBigPicture = (callback: () => void): Promise<UnlistenFn> =>
  hasBackend() ? listen("tray-big-picture", () => callback()) : Promise.resolve(noop);

export const onLibraryChanged = (callback: (games: Game[]) => void): Promise<UnlistenFn> =>
  hasBackend() ? listen<Game[]>("library-changed", (event) => callback(event.payload)) : Promise.resolve(noop);

export const onGameUpdated = (callback: (game: Game) => void): Promise<UnlistenFn> =>
  hasBackend() ? listen<Game>("game-updated", (event) => callback(event.payload)) : Promise.resolve(noop);

export const onGameRunning = (callback: (event: { id: string; running: boolean }) => void): Promise<UnlistenFn> =>
  hasBackend() ? listen<{ id: string; running: boolean }>("game-running", (event) => callback(event.payload)) : Promise.resolve(noop);

let mockInstallListener: ((event: InstallFinished) => void) | null = null;

/** In the browser preview the "installer" ends by itself a moment after it starts. */
export const onInstallFinished = (callback: (event: InstallFinished) => void): Promise<UnlistenFn> => {
  if (hasBackend()) return listen<InstallFinished>("install-finished", (event) => callback(event.payload));
  mockInstallListener = callback;
  window.setTimeout(
    () =>
      mockInstallListener?.({
        installId: "install:mock",
        prefixPath: "/mock/Prefixes/new",
        success: true,
        message: null,
        candidates: [
          { path: "/mock/Prefixes/new/drive_c/Games/Demo/demo.exe", shown: "Games/Demo/demo.exe", size: 48_000_000, score: 250 },
          { path: "/mock/Prefixes/new/drive_c/Games/Demo/editor.exe", shown: "Games/Demo/editor.exe", size: 9_000_000, score: 9 },
        ],
      }),
    1500,
  );
  return Promise.resolve(() => {
    mockInstallListener = null;
  });
};

export const onLaunchProgress = (callback: (progress: LaunchProgress) => void): Promise<UnlistenFn> =>
  hasBackend() ? listen<LaunchProgress>("launch-progress", (event) => callback(event.payload)) : Promise.resolve(noop);

export const onImageProgress = (callback: (progress: ImageProgress) => void): Promise<UnlistenFn> =>
  hasBackend() ? listen<ImageProgress>("images-progress", (event) => callback(event.payload)) : Promise.resolve(noop);
