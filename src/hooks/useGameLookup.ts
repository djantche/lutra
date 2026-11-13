import { useEffect, useState } from "react";
import { searchGameDb, type GameHit } from "../lib/backend";

export interface GameLookup {
  hits: GameHit[];
  loading: boolean;
  /** The lookup itself failed (offline...): the user can still fill the details in by hand. */
  failed: boolean;
}

const DEBOUNCE_MS = 450;

/** Searches GameDB for what the user is typing, once they pause. */
export function useGameLookup(query: string): GameLookup {
  const [state, setState] = useState<GameLookup>({ hits: [], loading: false, failed: false });
  const trimmed = query.trim();

  useEffect(() => {
    if (trimmed.length < 2) {
      setState({ hits: [], loading: false, failed: false });
      return;
    }
    let cancelled = false;
    setState((current) => ({ ...current, loading: true, failed: false }));
    const timer = window.setTimeout(() => {
      searchGameDb(trimmed)
        .then((hits) => !cancelled && setState({ hits, loading: false, failed: false }))
        .catch(() => !cancelled && setState({ hits: [], loading: false, failed: true }));
    }, DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [trimmed]);

  return state;
}
