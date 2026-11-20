import { useEffect, useRef } from "react";

/** How long to wait for the `[data-autofocus]` element to appear (a list still loading, a grid still measured). */
const WAIT_MS = 2000;

/**
 * Focuses the `[data-autofocus]` element inside the returned ref's container on mount. When it is
 * not there yet (the library has not arrived, a virtual grid has not drawn its first row), the
 * focus is given as soon as it shows up, unless the user already moved to something in the container.
 */
export function useAutofocus<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    const target = () => root.querySelector<HTMLElement>("[data-autofocus]");

    const first = target();
    if (first) {
      first.focus({ preventScroll: true });
      return;
    }

    const observer = new MutationObserver(() => {
      if (root.contains(document.activeElement) && document.activeElement !== root) return observer.disconnect();
      const found = target();
      if (!found) return;
      found.focus({ preventScroll: true });
      observer.disconnect();
    });
    observer.observe(root, { childList: true, subtree: true });
    const timer = window.setTimeout(() => observer.disconnect(), WAIT_MS);
    return () => {
      observer.disconnect();
      window.clearTimeout(timer);
    };
  }, []);
  return ref;
}
