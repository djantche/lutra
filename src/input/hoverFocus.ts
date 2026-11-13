import type { MouseEvent } from "react";

/** Spread on console-style focusables so pointing at an element selects it, like on a console. */
export const hoverFocus = {
  onMouseMove(event: MouseEvent<HTMLElement>) {
    // Browsers fire synthetic moves after scrolling; those have no movement and must not steal focus.
    if (event.movementX === 0 && event.movementY === 0) return;
    if (document.activeElement !== event.currentTarget) event.currentTarget.focus({ preventScroll: true });
  },
};
