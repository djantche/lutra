import type { TFunction } from "i18next";
import type { Game, GameSource } from "../types/game";
import { makerRank, platformOf } from "./platforms";

/** A page of the library: everything, a shortcut, a source, or the games of one console. */
export type LibraryFilter = "all" | "favorites" | "recent" | GameSource | `system:${string}`;

export const systemFilter = (system: string): LibraryFilter => `system:${system}`;

/** The console a filter is about, if it is a console's. */
export function filterSystem(filter: LibraryFilter): string | null {
  return filter.startsWith("system:") ? filter.slice("system:".length) : null;
}

/** The consoles of the emulated games in the library, with their game count, maker by maker. */
export function librarySystems(games: Game[]): { system: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const game of games) {
    if (game.system) counts.set(game.system, (counts.get(game.system) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([system, count]) => ({ system, count, platform: platformOf(system) }))
    .sort(
      (a, b) =>
        makerRank(a.platform.maker) - makerRank(b.platform.maker) ||
        (a.platform.year ?? 9999) - (b.platform.year ?? 9999) ||
        a.platform.name.localeCompare(b.platform.name),
    )
    .map(({ system, count }) => ({ system, count }));
}
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
    default: {
      const system = filterSystem(filter);
      return system
        ? games.filter((game) => game.system === system)
        : games.filter((game) => game.source === filter);
    }
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

/** What a filter's page is called: a console by its name, the rest translated. */
export function filterTitle(filter: LibraryFilter, t: TFunction): string {
  const system = filterSystem(filter);
  if (system) return platformOf(system).name;
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
