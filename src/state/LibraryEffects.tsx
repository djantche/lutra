import { useEffect } from "react";
import {
  hasBackend,
  listGames,
  onGameRunning,
  onGameUpdated,
  onImageProgress,
  onLaunchProgress,
  onLibraryChanged,
  runningGames,
} from "../lib/backend";
import type { Game } from "../types/game";
import { useLibraryStore } from "./library";

/** How often games updated by the backend are applied: artwork can arrive for thousands. */
const FLUSH_MS = 300;

/** Connects the library store to the backend: first load, scan, and live updates. Renders nothing. */
export function LibraryEffects() {
  useEffect(() => {
    if (!hasBackend()) return;
    const { setGames, upsertGames, setImageProgress, setRunning, rescan, applyLaunchProgress } =
      useLibraryStore.getState();

    // Updates are gathered and applied together, so the library is redrawn a few times a
    // second at most instead of once per game.
    const pending = new Map<string, Game>();
    let timer: number | undefined;
    const queueGame = (game: Game) => {
      pending.set(game.id, game);
      timer ??= window.setTimeout(() => {
        timer = undefined;
        const games = [...pending.values()];
        pending.clear();
        upsertGames(games);
      }, FLUSH_MS);
    };

    // Paint what we already know right away, then refresh from the sources.
    listGames()
      .then(setGames)
      .catch((error) => console.error("Reading the library failed:", error));
    rescan();
    runningGames()
      .then((ids) => ids.forEach((id) => setRunning(id, true)))
      .catch((error) => console.error("Reading running games failed:", error));

    // The backend pushes what happens on its own: artwork arriving, a game being closed.
    const stops = [onLibraryChanged(setGames), onGameUpdated(queueGame), onImageProgress(setImageProgress),
      onGameRunning(({ id, running }) => setRunning(id, running)),
      onLaunchProgress(applyLaunchProgress),
    ];
    return () => {
      window.clearTimeout(timer);
      stops.forEach((promise) => void promise.then((stop) => stop()));
    };
  }, []);

  return null;
}
