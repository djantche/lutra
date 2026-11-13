import type { InputAction } from "./actions";
import { playSound } from "../lib/sounds";
import { handleDefaultAction } from "./navigation";

/** Return `true` to consume the action and stop it from reaching other handlers. */
export type ActionHandler = (action: InputAction) => boolean | void;

const handlers: ActionHandler[] = [];

export function subscribe(handler: ActionHandler): () => void {
  handlers.push(handler);
  return () => {
    const index = handlers.indexOf(handler);
    if (index !== -1) handlers.splice(index, 1);
  };
}

/** Most recently subscribed handlers run first, so an open overlay wins over its page. */
export function emitAction(action: InputAction): void {
  for (let i = handlers.length - 1; i >= 0; i--) {
    if (handlers[i](action) === true) {
      // A screen took the action: it went back, opened something, or changed tab.
      if (action === "back") playSound("back");
      else if (action === "menu" || action === "options") playSound("open");
      else if (action === "tabNext" || action === "tabPrev") playSound("move");
      else if (action === "confirm") playSound("confirm");
      return;
    }
  }
  handleDefaultAction(action);
}
