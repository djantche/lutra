export type GameSource = "steam" | "gog" | "epic" | "wine" | "native";

export interface Game {
  id: string;
  title: string;
  source: GameSource;
  /** Base hue (0-360) used to generate placeholder artwork. */
  hue: number;
  // Real sources rarely know these; they stay empty until metadata providers exist.
  developer?: string;
  year?: number;
  /** `YYYY-MM-DD`, or just `YYYY`. */
  releaseDate?: string;
  /** IGDB id, when the details came from GameDB. */
  igdbId?: number;
  genres: string[];
  description?: string;
  /** Absolute path of a downloaded cover (portrait) and banner (wide); absent until imported. */
  poster?: string;
  hero?: string;
  /** Where the current cover and banner were downloaded from. */
  posterUrl?: string;
  heroUrl?: string;
  /** Absolute path of the downloaded logo (transparent title art), shown instead of the name. */
  logo?: string;
  /** Where the current logo came from. */
  logoUrl?: string;
  /** Changes when artwork is downloaded again, so cached copies get dropped. */
  artVersion?: number;
  /** SteamGridDB game picked by the user for this game's artwork. */
  sgdbGameId?: number;
  /** Set for games added by hand (Windows or Linux). */
  exePath?: string;
  runnerKind?: "wine" | "proton";
  playtimeMinutes: number;
  /** ISO date, or null if the game was never launched. */
  lastPlayed: string | null;
  favorite: boolean;
}
