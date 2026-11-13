import { create } from "zustand";
import {
  addGame as addGameBackend,
  hasBackend,
  importImages,
  initialGames,
  launchGameBackend,
  pickArtwork,
  removeGameBackend,
  scanSources,
  setFavorite,
  stopGameBackend,
  type ImageProgress,
  type LaunchProgress,
  type NewGame,
} from "../lib/backend";
import i18n from "../i18n";
import type { Game } from "../types/game";
import { reportError } from "./toasts";

// A launch the backend stopped reporting on (it always ends one way or another) is dropped after
// this long, so a lost event can never leave the interface stuck.
const LAUNCH_SILENCE_MS = 150_000;
let launchTimer: number | undefined;

/** Keeps the previous object of every game that did not change, so memoized cards skip re-rendering. */
function mergeGames(previous: Game[], incoming: Game[]): Game[] {
  const known = new Map(previous.map((game) => [game.id, game]));
  return incoming.map((game) => {
    const old = known.get(game.id);
    return old && JSON.stringify(old) === JSON.stringify(game) ? old : game;
  });
}

export interface LaunchStage {
  id: string;
  stage: LaunchProgress["stage"];
  /** Stages finished so far, out of `steps`. */
  step: number;
  steps: number;
}

interface LibraryState {
  games: Game[];
  /** True while sources are being scanned. */
  scanning: boolean;
  /** Artwork download progress; `running` is false when idle. */
  imageProgress: ImageProgress;
  /** The launch in progress, with the real stage the backend reported last. */
  launch: LaunchStage | null;
  /** Games currently running. */
  runningIds: string[];

  setGames: (games: Game[]) => void;
  /** Replaces one game (or adds it), leaving every other game untouched. */
  upsertGame: (game: Game) => void;
  /** Several games at once, in a single update (artwork arriving for a large library). */
  upsertGames: (games: Game[]) => void;
  setImageProgress: (progress: ImageProgress) => void;
  toggleFavorite: (id: string) => void;
  launchGame: (id: string) => void;
  /** Applies what the backend reported about a launch. */
  applyLaunchProgress: (progress: LaunchProgress) => void;
  stopGame: (id: string) => void;
  setRunning: (id: string, running: boolean) => void;
  /** Downloads a game's artwork again, keeping the SteamGridDB game the user picked, if any. */
  refreshArtwork: (id: string) => void;
  rescan: () => void;
  addGame: (input: NewGame) => Promise<Game>;
  removeGame: (id: string, everything?: boolean) => Promise<void>;
}

function armLaunchGuard(set: (state: Partial<LibraryState>) => void) {
  window.clearTimeout(launchTimer);
  launchTimer = window.setTimeout(() => set({ launch: null }), LAUNCH_SILENCE_MS);
}

const fetchArtwork = () => {
  // A no-op without anything to fetch.
  void importImages(false).catch((error) => console.error("Importing artwork failed:", error));
};

export const useLibraryStore = create<LibraryState>((set, get) => ({
  games: initialGames(),
  scanning: hasBackend(),
  imageProgress: { done: 0, total: 0, running: false },
  launch: null,
  runningIds: [],

  setGames: (games) => set((state) => ({ games: mergeGames(state.games, games) })),

  upsertGame: (game) =>
    set((state) => ({
      games: state.games.some((candidate) => candidate.id === game.id)
        ? state.games.map((candidate) => (candidate.id === game.id ? game : candidate))
        : [...state.games, game],
    })),

  upsertGames: (updated) =>
    set((state) => {
      const byId = new Map(updated.map((game) => [game.id, game]));
      const games = state.games.map((game) => {
        const next = byId.get(game.id);
        if (next) byId.delete(game.id);
        return next ?? game;
      });
      return { games: byId.size > 0 ? [...games, ...byId.values()] : games };
    }),

  setImageProgress: (imageProgress) => set({ imageProgress }),

  toggleFavorite: (id) => {
    const game = get().games.find((candidate) => candidate.id === id);
    if (!game) return;
    get().upsertGame({ ...game, favorite: !game.favorite });
    if (hasBackend()) {
      setFavorite(id, !game.favorite).catch((error) => reportError(i18n.t("errors.favorite"), error));
    }
  },

  launchGame: (id) => {
    const game = get().games.find((candidate) => candidate.id === id);
    if (!game || get().launch) return;
    get().upsertGame({ ...game, lastPlayed: new Date().toISOString() });
    if (!hasBackend()) return;

    set({ launch: { id, stage: "check", step: 0, steps: 3 } });
    armLaunchGuard(set);
    launchGameBackend(id).catch((error) => {
      window.clearTimeout(launchTimer);
      set({ launch: null });
      reportError(i18n.t("errors.launch", { title: game.title }), error);
    });
  },

  applyLaunchProgress: (progress) => {
    const current = get().launch;
    // The progress of a launch this window did not start (the overlay, a second window) still counts.
    if (current && current.id !== progress.id) return;
    if (progress.state === "progress") {
      set({ launch: { id: progress.id, stage: progress.stage, step: progress.step, steps: progress.steps } });
      armLaunchGuard(set);
      return;
    }
    window.clearTimeout(launchTimer);
    set({ launch: null });
    if (progress.state === "failed" && progress.message) {
      const title = get().games.find((game) => game.id === progress.id)?.title ?? "";
      reportError(i18n.t("errors.launch", { title }), progress.message);
    }
  },

  stopGame: (id) => {
    stopGameBackend(id).catch((error) => reportError(i18n.t("errors.stop"), error));
  },

  setRunning: (id, running) =>
    set((state) => ({
      runningIds: running
        ? state.runningIds.includes(id)
          ? state.runningIds
          : [...state.runningIds, id]
        : state.runningIds.filter((candidate) => candidate !== id),
    })),

  refreshArtwork: (id) => {
    const game = get().games.find((candidate) => candidate.id === id);
    pickArtwork(id, game?.sgdbGameId ?? null).catch((error) => reportError(i18n.t("errors.artwork"), error));
  },

  rescan: () => {
    if (!hasBackend()) return;
    set({ scanning: true });
    scanSources()
      .then((found) => {
        get().setGames(found);
        fetchArtwork();
      })
      .catch((error) => reportError(i18n.t("errors.scan"), error))
      .finally(() => set({ scanning: false }));
  },

  addGame: async (input) => {
    const game = await addGameBackend(input);
    get().upsertGame(game);
    fetchArtwork();
    return game;
  },

  removeGame: async (id, everything = false) => {
    await removeGameBackend(id, everything);
    set((state) => ({ games: state.games.filter((game) => game.id !== id) }));
  },
}));

/** Whole store plus the game being launched. Prefer `useLibraryStore(selector)` when only a slice is needed. */
export function useLibrary() {
  const state = useLibraryStore();
  const launching = state.games.find((game) => game.id === state.launch?.id) ?? null;
  return { ...state, launching };
}
