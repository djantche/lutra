import type { GameMeta } from "../../lib/backend";
import type { Game } from "../../types/game";

/** The editable form of a game's details: everything is text until it is saved. */
export interface DetailsForm {
  developer: string;
  releaseDate: string;
  /** Comma-separated. */
  genres: string;
  description: string;
  igdbId: number | null;
}

export const EMPTY_DETAILS: DetailsForm = { developer: "", releaseDate: "", genres: "", description: "", igdbId: null };

export const detailsFromMeta = (meta: GameMeta): DetailsForm => ({
  developer: meta.developer ?? "",
  releaseDate: meta.releaseDate ?? "",
  genres: meta.genres.join(", "),
  description: meta.description ?? "",
  igdbId: meta.igdbId,
});

export const detailsFromGame = (game: Game): DetailsForm => ({
  developer: game.developer ?? "",
  releaseDate: game.releaseDate ?? (game.year ? String(game.year) : ""),
  genres: game.genres.join(", "),
  description: game.description ?? "",
  igdbId: game.igdbId ?? null,
});

/** `YYYY` or `YYYY-MM-DD`, or empty. */
export const isValidDate = (value: string): boolean => {
  const text = value.trim();
  if (text === "") return true;
  const match = /^(\d{4})(?:-(\d{2})-(\d{2}))?$/.exec(text);
  if (!match) return false;
  if (!match[2]) return true;
  const date = new Date(`${text}T00:00:00Z`);
  return !Number.isNaN(date.getTime()) && date.toISOString().startsWith(text);
};

const orNull = (text: string) => (text.trim() === "" ? null : text.trim());

export const metaFromDetails = (form: DetailsForm): GameMeta => ({
  developer: orNull(form.developer),
  releaseDate: orNull(form.releaseDate),
  genres: form.genres
    .split(",")
    .map((genre) => genre.trim())
    .filter((genre) => genre !== ""),
  description: orNull(form.description),
  igdbId: form.igdbId,
});
