import type { TFunction } from "i18next";
import type { Game, GameSource } from "../types/game";

/** A page of the library: everything, a shortcut, or one source. */
export type LibraryFilter = "all" | "favorites" | "recent" | GameSource;

export type SortKey = "name" | "lastPlayed" | "playtime";

export function recentGames(games: Game[], limit = Infinity): Game[] {
  return games
    .filter((game) => game.lastPlayed !== null)
    .sort((a, b) => Date.parse(b.lastPlayed!) - Date.parse(a.lastPlayed!))
    .slice(0, limit);
}

export function filterGames(games: Game[], filter: LibraryFilter): Game[] {
  switch (filter) {
    case "all":
      return games;
    case "favorites":
      return games.filter((game) => game.favorite);
    case "recent":
      return recentGames(games, 10);
    default:
      return games.filter((game) => game.source === filter);
  }
}

export function sortGames(games: Game[], key: SortKey): Game[] {
  const sorted = [...games];
  switch (key) {
    case "name":
      return sorted.sort((a, b) => a.title.localeCompare(b.title));
    case "playtime":
      return sorted.sort((a, b) => b.playtimeMinutes - a.playtimeMinutes);
    case "lastPlayed":
      return sorted.sort((a, b) => Date.parse(b.lastPlayed ?? "0") - Date.parse(a.lastPlayed ?? "0"));
  }
}

/** What a filter's page is called. */
export function filterTitle(filter: LibraryFilter, t: TFunction): string {
  switch (filter) {
    case "all":
      return t("nav.allGames");
    case "favorites":
      return t("nav.favorites");
    case "recent":
      return t("nav.recent");
    default:
      return t(`sources.${filter}`);
  }
}
