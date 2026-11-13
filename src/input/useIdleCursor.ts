import { useEffect, useState } from "react";

/** True once the mouse has been still for `delay` ms, or as soon as a key is pressed. */
export function useIdleCursor(delay = 2500): boolean {
  const [idle, setIdle] = useState(false);

  useEffect(() => {
    let timer: number | undefined;
    const wake = (event: MouseEvent) => {
      if (event.movementX === 0 && event.movementY === 0) return;
      setIdle(false);
      window.clearTimeout(timer);
      timer = window.setTimeout(() => setIdle(true), delay);
    };
    const sleep = () => {
      window.clearTimeout(timer);
      setIdle(true);
    };

    timer = window.setTimeout(() => setIdle(true), delay);
    window.addEventListener("mousemove", wake);
    window.addEventListener("keydown", sleep);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("mousemove", wake);
      window.removeEventListener("keydown", sleep);
    };
  }, [delay]);

  return idle;
}
