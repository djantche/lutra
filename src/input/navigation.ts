import { playSound } from "../lib/sounds";
import type { InputAction } from "./actions";

type Direction = "up" | "down" | "left" | "right";

const FOCUSABLE = "[data-focusable]:not([disabled])";

// What moved the selection last: the keyboard or the pad (`nav`), or the pointer. A focus given by a
// script after a click is not "focus-visible" to the browser, so styles follow this instead.
const setInput = (kind: "nav" | null) => {
  if (kind) document.documentElement.dataset.input = kind;
  else delete document.documentElement.dataset.input;
};
if (typeof window !== "undefined") {
  window.addEventListener("pointerdown", () => setInput(null), true);
}

/** A row of a menu: one item to move across, which acts on the control inside it. */
export const ROW = "[data-nav-row]";

/**
 * What navigation lands on for a focusable: its row when that row holds a single control (a
 * toggle, a list, a field), like the lines of a game's menu; the control itself otherwise (a row
 * of several choices keeps one stop per choice).
 */
function stop(el: HTMLElement): HTMLElement {
  const row = el.closest<HTMLElement>(ROW);
  return row && row.querySelectorAll(FOCUSABLE).length === 1 ? row : el;
}

/** The innermost modal container if any, otherwise the whole document. */
function navigationRoot(): ParentNode {
  const traps = document.querySelectorAll("[data-nav-trap]");
  return traps.length > 0 ? traps[traps.length - 1] : document;
}

function isVisible(el: HTMLElement): boolean {
  return el.getClientRects().length > 0 && getComputedStyle(el).visibility !== "hidden";
}

const intervalGap = (a0: number, a1: number, b0: number, b1: number) =>
  Math.max(0, a0 - b1, b0 - a1);

/** Lower is better; Infinity means the candidate is not in that direction. */
function score(direction: Direction, from: DOMRect, to: DOMRect): number {
  const dx = to.left + to.width / 2 - (from.left + from.width / 2);
  const dy = to.top + to.height / 2 - (from.top + from.height / 2);

  let ahead: boolean;
  let gap: number;
  let cross: number;
  let crossCenter: number;

  switch (direction) {
    case "right":
      ahead = dx > 1;
      gap = Math.max(0, to.left - from.right);
      cross = intervalGap(from.top, from.bottom, to.top, to.bottom);
      crossCenter = Math.abs(dy);
      break;
    case "left":
      ahead = dx < -1;
      gap = Math.max(0, from.left - to.right);
      cross = intervalGap(from.top, from.bottom, to.top, to.bottom);
      crossCenter = Math.abs(dy);
      break;
    case "down":
      ahead = dy > 1;
      gap = Math.max(0, to.top - from.bottom);
      cross = intervalGap(from.left, from.right, to.left, to.right);
      crossCenter = Math.abs(dx);
      break;
    case "up":
      ahead = dy < -1;
      gap = Math.max(0, from.top - to.bottom);
      cross = intervalGap(from.left, from.right, to.left, to.right);
      crossCenter = Math.abs(dx);
      break;
  }

  return ahead ? gap + cross * 3 + crossCenter * 0.25 : Infinity;
}

function moveFocus(direction: Direction): void {
  const root = navigationRoot();
  const items = [...new Set(Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).map(stop))].filter(isVisible);
  if (items.length === 0) return;

  const active = document.activeElement;
  const here = active instanceof HTMLElement ? stop(active) : null;
  const current = here && items.includes(here) ? here : null;
  if (!current) {
    (root.querySelector<HTMLElement>("[data-autofocus]") ?? items[0]).focus();
    return;
  }

  const from = current.getBoundingClientRect();
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const item of items) {
    if (item === current) continue;
    const s = score(direction, from, item.getBoundingClientRect());
    if (s < bestScore) {
      best = item;
      bestScore = s;
    }
  }
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const behavior: ScrollBehavior = reduceMotion ? "auto" : "smooth";
  if (!best) {
    // Nothing left to select that way: the list still shows what is past its last item.
    scrollToEdge(current, direction, behavior);
    return;
  }

  setInput("nav");
  best.focus({ preventScroll: true });
  playSound("move");
  best.scrollIntoView({
    block: "nearest",
    inline: (best.dataset.scrollInline as ScrollLogicalPosition | undefined) ?? "nearest",
    behavior,
  });
}

/** The nearest ancestor that scrolls vertically. */
function scrollParent(el: HTMLElement): HTMLElement | null {
  for (let parent = el.parentElement; parent; parent = parent.parentElement) {
    const { overflowY } = getComputedStyle(parent);
    if ((overflowY === "auto" || overflowY === "scroll") && parent.scrollHeight > parent.clientHeight) return parent;
  }
  return null;
}

/** Down at the last item goes to the end of the list, up at the first one to its start. */
function scrollToEdge(from: HTMLElement, direction: Direction, behavior: ScrollBehavior): void {
  if (direction !== "up" && direction !== "down") return;
  const parent = scrollParent(from);
  if (!parent) return;
  const top = direction === "down" ? parent.scrollHeight - parent.clientHeight : 0;
  if (Math.abs(parent.scrollTop - top) > 1) parent.scrollTo({ top, behavior });
}

/** Default behaviour when no screen consumed the action. */
export function handleDefaultAction(action: InputAction): void {
  switch (action) {
    case "up":
    case "down":
    case "left":
    case "right":
      moveFocus(action);
      break;
    case "confirm": {
      const active = document.activeElement;
      if (!(active instanceof HTMLElement)) break;
      const control = active.matches(ROW) ? active.querySelector<HTMLElement>(FOCUSABLE) : active;
      if (control instanceof HTMLInputElement || control instanceof HTMLTextAreaElement) control.focus();
      else control?.click();
      // A switch makes its own sound.
      if (control?.getAttribute("role") !== "switch") playSound("confirm");
      break;
    }
    case "back": {
      const active = document.activeElement;
      if (!(active instanceof HTMLInputElement)) break;
      // Leaving a field goes back to its row, so navigation carries on from there.
      const row = active.closest<HTMLElement>(ROW);
      if (row) row.focus({ preventScroll: true });
      else active.blur();
      playSound("back");
      break;
    }
  }
}
