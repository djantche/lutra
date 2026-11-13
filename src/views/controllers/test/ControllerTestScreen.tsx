import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { GamepadVisual } from "../../../components/GamepadVisual/GamepadVisual";
import { Icon } from "../../../components/Icon/Icon";
import { useInputAction } from "../../../input/useInputAction";
import {
  controllerPreview,
  type ControllerButton,
  type ControllerInfo,
  type ControllerProfile,
  type PadPreview,
} from "../../../lib/controllers";
import { ButtonCallouts } from "./ButtonCallouts";
import { HoldToExit } from "./HoldToExit";
import { StickPlot } from "./StickPlot";
import { TriggerMeter } from "./TriggerMeter";
import styles from "./ControllerTestScreen.module.scss";

const REFRESH_MS = 33;

const LEFT: ControllerButton[] = ["l2", "l1", "dpadUp", "dpadLeft", "dpadRight", "dpadDown", "l3", "select"];
const RIGHT: ControllerButton[] = ["r2", "r1", "north", "west", "east", "south", "r3", "start", "guide"];

interface ControllerTestScreenProps {
  controller: ControllerInfo;
  profile: ControllerProfile;
  onClose: () => void;
}

/**
 * Full-screen test of a controller with a profile: every press shows what the game would get.
 * While it is open the controller does not drive the interface; holding the back button leaves.
 */
export function ControllerTestScreen({ controller, profile, onClose }: ControllerTestScreenProps) {
  const { t } = useTranslation();
  const [preview, setPreview] = useState<PadPreview | null>(null);

  useEffect(() => {
    let current = true;
    const timer = window.setInterval(() => {
      controllerPreview(controller.id, profile)
        .then((next) => current && setPreview(next))
        .catch(() => {});
    }, REFRESH_MS);
    return () => {
      current = false;
      window.clearInterval(timer);
    };
  }, [controller.id, profile]);

  // Every controller (and keyboard) action is swallowed: pressing buttons is the point here.
  useInputAction(() => true);

  // Keyboard users leave with Escape.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const raw = preview?.raw ?? null;
  const output = preview?.output ?? null;
  const host = document.querySelector("[data-overlay-root]") ?? document.body;

  return createPortal(
    <div className={styles.screen} role="dialog" aria-modal="true" aria-label={t("controllers.test.title")} data-nav-trap>
      <header className={styles.header}>
        <div>
          <h2>{t("controllers.test.title")}</h2>
          <p>
            {t(`controllers.family.${controller.family}`)} ·{" "}
            {t("controllers.test.sees", { output: t(`controllers.output.${profile.output}`) })}
          </p>
        </div>
        <button type="button" className={styles.close} aria-label={t("controllers.test.close")} onClick={onClose}>
          <Icon name="close" size={20} />
        </button>
      </header>

      <div className={styles.stage}>
        <ButtonCallouts side="left" buttons={LEFT} profile={profile} family={controller.family} pressed={raw?.pressed ?? []} />
        <div className={styles.pad}>
          <GamepadVisual state={raw} family={controller.family} model={controller.model} />
        </div>
        <ButtonCallouts side="right" buttons={RIGHT} profile={profile} family={controller.family} pressed={raw?.pressed ?? []} />
      </div>

      <div className={styles.analog}>
        <StickPlot
          label={t("controllers.test.leftStick")}
          raw={raw ? [raw.lx, raw.ly] : [0, 0]}
          output={output ? [output.lx, output.ly] : [0, 0]}
          deadzone={profile.leftDeadzone}
        />
        <TriggerMeter label={t("controllers.test.leftTrigger")} raw={raw?.lt ?? 0} output={output?.lt ?? 0} />
        <TriggerMeter label={t("controllers.test.rightTrigger")} raw={raw?.rt ?? 0} output={output?.rt ?? 0} />
        <StickPlot
          label={t("controllers.test.rightStick")}
          raw={raw ? [raw.rx, raw.ry] : [0, 0]}
          output={output ? [output.rx, output.ry] : [0, 0]}
          deadzone={profile.rightDeadzone}
        />
      </div>

      <HoldToExit holding={raw?.pressed.includes("east") ?? false} family={controller.family} onExit={onClose} />
    </div>,
    host,
  );
}
