import { useEffect, useState } from "react";
import { lockInput } from "../../input/bus";
import styles from "./Splash.module.scss";

/**
 * Brand intro played when entering Big Picture; removes itself once the animation ends. While it
 * plays the interface cannot be used: the pointer is stopped by the intro itself, the pad and the
 * keyboard are held back.
 */
export function Splash() {
  const [done, setDone] = useState(false);

  useEffect(() => {
    if (done) return;
    const unlock = lockInput();
    const stop = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
    };
    window.addEventListener("keydown", stop, true);
    window.addEventListener("keyup", stop, true);
    return () => {
      unlock();
      window.removeEventListener("keydown", stop, true);
      window.removeEventListener("keyup", stop, true);
    };
  }, [done]);

  if (done) return null;

  return (
    <div
      className={styles.splash}
      aria-hidden="true"
      onAnimationEnd={(event) => {
        if (event.target === event.currentTarget) setDone(true);
      }}
    >
      <div className={styles.mark}>
        <img src="/lutra.svg" alt="" width={96} height={96} />
        <span>Lutra</span>
      </div>
    </div>
  );
}
