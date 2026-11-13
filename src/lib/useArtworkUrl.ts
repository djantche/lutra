import { useEffect, useState } from "react";
import { readArtwork } from "./backend";

const MIME: Record<string, string> = { png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp" };

interface Entry {
  version: number | undefined;
  url: Promise<string>;
  /** Set once loaded, so later renders get the image synchronously. */
  ready?: string;
}

// One object URL per image file, replaced (and the old one released) when its version changes.
const entries = new Map<string, Entry>();

function load(path: string, version: number | undefined): Promise<string> {
  const known = entries.get(path);
  if (known && known.version === version) return known.url;

  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  const entry: Entry = {
    version,
    url: readArtwork(path).then((bytes) => {
      const url = URL.createObjectURL(new Blob([bytes], { type: MIME[extension] ?? "application/octet-stream" }));
      entry.ready = url;
      return url;
    }),
  };
  entries.set(path, entry);
  if (known) void known.url.then((old) => URL.revokeObjectURL(old));
  return entry.url;
}

/** The image's URL if it was already loaded once: lets a remounted card show it immediately. */
export function cachedArtworkUrl(path: string | undefined, version: number | undefined): string | null {
  if (!path) return null;
  const entry = entries.get(path);
  return entry && entry.version === version ? (entry.ready ?? null) : null;
}

/**
 * Local URL of a downloaded image, or null while it loads (or if there is none).
 * Images go through the backend instead of the webview reading files itself.
 */
export function useArtworkUrl(path: string | undefined, version: number | undefined): string | null {
  const [url, setUrl] = useState<string | null>(() => cachedArtworkUrl(path, version));

  useEffect(() => {
    const cached = cachedArtworkUrl(path, version);
    if (cached || !path) {
      setUrl(cached);
      return;
    }
    let current = true;
    load(path, version)
      .then((loadedUrl) => current && setUrl(loadedUrl))
      .catch((error) => {
        console.error("Loading artwork failed:", error);
        if (current) setUrl(null);
      });
    return () => {
      current = false;
    };
  }, [path, version]);

  return url;
}
