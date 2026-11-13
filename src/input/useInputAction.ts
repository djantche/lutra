import { useEffect, useRef } from "react";
import { subscribe, type ActionHandler } from "./bus";

export function useInputAction(handler: ActionHandler, enabled = true): void {
  const latest = useRef(handler);
  useEffect(() => {
    latest.current = handler;
  });

  useEffect(() => {
    if (!enabled) return;
    return subscribe((action) => latest.current(action));
  }, [enabled]);
}
