import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BatteryGauge } from "../../components/BatteryGauge/BatteryGauge";
import { Button } from "../../components/Button/Button";
import { GamepadVisual } from "../../components/GamepadVisual/GamepadVisual";
import {
  TEST_SESSION,
  type ControllerInfo,
  type ControllerProfile,
  type ControllerStatus,
  type PadSnapshot,
} from "../../lib/controllers";
import { useLibraryStore } from "../../state/library";
import { ControllerTestScreen } from "./test/ControllerTestScreen";
import styles from "./Controllers.module.scss";

interface LiveTesterProps {
  controller: ControllerInfo;
  state: PadSnapshot | null;
  status: ControllerStatus;
  profile: ControllerProfile;
}

/** The controller drawn live, who currently drives it, and the way into the test screen. */
export function LiveTester({ controller, state, status, profile }: LiveTesterProps) {
  const { t } = useTranslation();
  const [testing, setTesting] = useState(false);
  const session = status.session;
  const owner = useLibraryStore((store) => store.games.find((game) => game.id === session?.owner));
  const inGame = session !== null && session.owner !== TEST_SESSION;
  // Outside games: driven system-wide by Lutra, left to Steam Input, or left as it is.
  const idle = !status.system || !status.uinput ? "idle" : status.steam ? "steam" : "system";

  return (
    <section className={styles.tester}>
      <GamepadVisual state={state} family={controller.family} />

      <div className={styles.session}>
        <span className={styles.dot} data-active={inGame || idle === "system"} />
        <div className={styles.sessionText}>
          <strong>
            {inGame
              ? t("controllers.emulating", { output: t(`controllers.output.${session.output}`) })
              : t(`controllers.state.${idle}`)}
          </strong>
          <small>
            {inGame ? t("controllers.forGame", { title: owner?.title ?? session.owner }) : t(`controllers.state.${idle}Hint`)}
          </small>
        </div>
        {controller.battery && <BatteryGauge battery={controller.battery} />}
        <Button variant="primary" icon="gamepad" disabled={inGame} onClick={() => setTesting(true)}>
          {t("controllers.test.open")}
        </Button>
      </div>

      {testing && <ControllerTestScreen controller={controller} profile={profile} onClose={() => setTesting(false)} />}
    </section>
  );
}
