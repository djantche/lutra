import { useState } from "react";
import styles from "./Splash.module.scss";

/** Brand intro played when entering Big Picture; removes itself once the animation ends. */
export function Splash() {
  const [done, setDone] = useState(false);
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
