import type { TFunction } from "i18next";
import type { PrefixInfo } from "./backend";

/** The most a prefix's name can hold (the backend refuses more). */
export const PREFIX_NAME_MAX = 60;

/** Games of a prefix as one line: "Sifu", "Sifu, Hades", "Sifu, Hades + 2 more". */
export function gamesLine(games: string[], t: TFunction): string {
  if (games.length <= 2) return games.join(", ");
  return `${games.slice(0, 2).join(", ")} ${t("prefix.more", { count: games.length - 2 })}`;
}

/**
 * What a prefix is called: the name the user gave it, otherwise its games, otherwise what the
 * folder is called (Steam's prefixes already carry their game's name; an unused Lutra one only has
 * the folder's code, which says nothing).
 */
export function prefixTitle(prefix: PrefixInfo, t: TFunction): string {
  if (prefix.customName) return prefix.customName;
  if (prefix.games.length > 0) return gamesLine(prefix.games, t);
  if (prefix.origin === "lutra") return t("prefix.unused", { code: prefix.name });
  return prefix.name;
}
