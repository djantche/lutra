import { useEffect, useRef } from "react";

/** Focuses the `[data-autofocus]` element inside the returned ref's container on mount. */
export function useAutofocus<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus({ preventScroll: true });
  }, []);
  return ref;
}
