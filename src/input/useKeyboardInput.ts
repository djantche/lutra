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

const TEXT_FIELD = "input:not([type=button]):not([type=checkbox]):not([type=radio]), textarea, select";

export function useKeyboardInput(): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
      const action = KEY_MAP[event.key];
      if (!action) return;

      const target = event.target;
      const typing = target instanceof HTMLElement && (target.matches(TEXT_FIELD) || target.isContentEditable);
      if (typing && action !== "back" && action !== "toggleMode" && action !== "menu" && action !== "options") return;

      event.preventDefault();
      emitAction(action);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
