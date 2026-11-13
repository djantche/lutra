import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { GameArt } from "../../components/GameArt/GameArt";
import { GameLogo } from "../../components/GameLogo/GameLogo";
import { LaunchRing } from "../../components/LaunchRing/LaunchRing";
import { useInputAction } from "../../input/useInputAction";
import { launchLabel } from "../../lib/launch";
import { useLibraryStore } from "../../state/library";
import type { Game } from "../../types/game";
import styles from "./LaunchScreen.module.scss";

export function LaunchScreen({ game }: { game: Game }) {
  const { t } = useTranslation();
  const launch = useLibraryStore((state) => state.launch);

  // Swallow input while the game starts.
  useInputAction(() => true);

  return (
    <div className={styles.launch} style={{ "--hue": game.hue } as CSSProperties} role="status">
      <div className={styles.art} aria-hidden="true">
        <GameArt game={game} kind="hero" />
      </div>
      <div className={styles.center}>
        <GameLogo game={game} className={styles.logo} />
        {launch && <LaunchRing step={launch.step} steps={launch.steps} size={72} />}
        <p>{launchLabel(t, launch)}</p>
      </div>
    </div>
  );
}
