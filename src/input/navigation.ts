import type { InputAction } from "./actions";

type Direction = "up" | "down" | "left" | "right";

const FOCUSABLE = "[data-focusable]:not([disabled])";

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
  const items = Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(isVisible);
  if (items.length === 0) return;

  const active = document.activeElement;
  const current = active instanceof HTMLElement && items.includes(active) ? active : null;
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
  if (!best) return;

  best.focus({ preventScroll: true });
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  best.scrollIntoView({
    block: "nearest",
    inline: (best.dataset.scrollInline as ScrollLogicalPosition | undefined) ?? "nearest",
    behavior: reduceMotion ? "auto" : "smooth",
  });
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
    case "confirm":
      if (document.activeElement instanceof HTMLElement) document.activeElement.click();
      break;
    case "back":
      if (document.activeElement instanceof HTMLInputElement) document.activeElement.blur();
      break;
  }
}
