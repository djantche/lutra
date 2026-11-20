import { isHosted } from "./host";

/**
 * Address of a downloaded image, or null when there is none (or no shell to serve it). The shell
 * serves the files of the artwork folder under `lutra://art/`, so the address is known at once:
 * a card never waits for a round trip to the backend, and the browser loads and caches the image
 * itself. `version` changes the address when the image is replaced.
 */
export function artworkUrl(path: string | undefined, version: number | undefined): string | null {
  if (!path || !isHosted()) return null;
  return `lutra://art/?f=${encodeURIComponent(path)}&v=${version ?? 0}`;
}

/** The image's address (see `artworkUrl`). */
export function useArtworkUrl(path: string | undefined, version: number | undefined): string | null {
  return artworkUrl(path, version);
}
