import { useEffect, useState } from "react";
import { coreOptions, type CoreOptions } from "../lib/backend";

// A core's options do not change while Lutra runs: read once per console.
const cache = new Map<string, Promise<CoreOptions>>();

/** The options the emulator of `system` offers; null while they are read. */
export function useCoreOptions(system: string) {
  const [options, setOptions] = useState<CoreOptions | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    let request = cache.get(system);
    if (!request) {
      request = coreOptions(system);
      cache.set(system, request);
      // A failure is not kept: the emulator may be installed a moment later.
      request.catch(() => cache.delete(system));
    }
    request
      .then((found) => current && setOptions(found))
      .catch((reason) => current && setError(String(reason)));
    return () => {
      current = false;
    };
  }, [system]);

  return { options, error };
}
