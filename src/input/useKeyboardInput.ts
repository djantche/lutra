import { useEffect } from "react";
import type { InputAction } from "./actions";
import { emitAction } from "./bus";

// Enter is left to the browser: it already clicks the focused button.
const KEY_MAP: Record<string, InputAction> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Escape: "back",
  PageUp: "tabPrev",
  PageDown: "tabNext",
  F11: "toggleMode",
  F1: "menu",
  F2: "options",
};

const ROW = "[data-nav-row]";
const TEXT_FIELD = "input:not([type=button]):not([type=checkbox]):not([type=radio]), textarea, select";

export function useKeyboardInput(): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      // A focused row of a menu acts on its control, which the browser would not do by itself.
      const row = event.target instanceof HTMLElement && event.target.matches(ROW);
      const action = row && (event.key === "Enter" || event.key === " ") ? "confirm" : KEY_MAP[event.key];
      if (!action) return;

      const target = event.target;
      const typing = target instanceof HTMLElement && (target.matches(TEXT_FIELD) || target.isContentEditable);
      // A single-line field has no use for up and down: they move to the next or previous item, so
      // the keyboard never gets stuck in a field. Left and right stay with the caret.
      const leavesField = target instanceof HTMLInputElement && (action === "up" || action === "down");
      if (typing && !leavesField && action !== "back" && action !== "toggleMode" && action !== "menu" && action !== "options") return;

      event.preventDefault();
      emitAction(action);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
