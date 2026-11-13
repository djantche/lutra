import { useTranslation } from "react-i18next";
import { GameArt } from "../../components/GameArt/GameArt";
import { LaunchRing } from "../../components/LaunchRing/LaunchRing";
import { launchLabel } from "../../lib/launch";
import { useLibrary } from "../../state/library";
import styles from "./LaunchToast.module.scss";

/** Shown for as long as a launch lasts, with the stage the backend is really at. */
export function LaunchToast() {
  const { t } = useTranslation();
  const { launching, launch } = useLibrary();
  if (!launching || !launch) return null;

  return (
    <div key={launching.id} className={styles.toast} role="status">
      <span className={styles.thumb}>
        <GameArt game={launching} />
      </span>
      <span className={styles.text}>
        <small>{launchLabel(t, launch)}</small>
        <strong>{launching.title}</strong>
      </span>
      <LaunchRing step={launch.step} steps={launch.steps} size={28} />
    </div>
  );
}
