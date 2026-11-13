import { useCallback, useEffect, useRef, useState } from "react";
import {
  DEFAULT_PROFILE,
  controllerProfile,
  controllerStatus,
  onControllersChanged,
  resetControllerProfile,
  saveControllerProfile,
  type ControllerBattery,
  type ControllerFamily,
  type ControllerProfile,
  type ControllerStatus,
} from "../../lib/controllers";
import i18n from "../../i18n";
import { reportError } from "../../state/toasts";

const STATUS_REFRESH_MS = 2000;
const SAVE_DELAY_MS = 350;

/** Connected controllers and who holds them, kept fresh while the screen is open. */
export function useControllerStatus() {
  const [status, setStatus] = useState<ControllerStatus | null>(null);

  const refresh = useCallback(() => {
    controllerStatus()
      .then(setStatus)
      .catch((error) => console.error("Reading controllers failed:", error));
  }, []);

  useEffect(() => {
    refresh();
    const timer = window.setInterval(refresh, STATUS_REFRESH_MS);
    const stop = onControllersChanged(refresh);
    return () => {
      window.clearInterval(timer);
      void stop.then((unlisten) => unlisten());
    };
  }, [refresh]);

  return { status, refresh };
}

const BATTERY_REFRESH_MS = 15000;

/** A controller whose charge is known, with what is needed to tell it from the others. */
export interface ControllerCharge {
  id: string;
  name: string;
  family: ControllerFamily;
  battery: ControllerBattery;
}

/** The charge of every connected controller that reports one, for a corner of the screen. */
export function useControllerBatteries(): ControllerCharge[] {
  const [batteries, setBatteries] = useState<ControllerCharge[]>([]);

  useEffect(() => {
    const read = () =>
      controllerStatus()
        .then((status) =>
          setBatteries(
            status.controllers.flatMap((controller) =>
              controller.battery
                ? [{ id: controller.id, name: controller.name, family: controller.family, battery: controller.battery }]
                : [],
            ),
          ),
        )
        .catch(() => {});
    read();
    const timer = window.setInterval(read, BATTERY_REFRESH_MS);
    const stop = onControllersChanged(read);
    return () => {
      window.clearInterval(timer);
      void stop.then((unlisten) => unlisten());
    };
  }, []);

  return batteries;
}

/**
 * The default profile (`game: null`) or a game's own. Every change is saved on its own after a
 * short pause, so there is no Save button to forget.
 */
export function useControllerProfile(game: string | null) {
  const [profile, setProfile] = useState<ControllerProfile | null>(null);
  const [custom, setCustom] = useState(false);
  const pending = useRef<number | undefined>(undefined);

  useEffect(() => {
    let current = true;
    controllerProfile(game)
      .then((loaded) => {
        if (!current) return;
        setProfile(loaded.profile);
        setCustom(loaded.custom);
      })
      .catch((error) => reportError(i18n.t("errors.save"), error));
    return () => {
      current = false;
    };
  }, [game]);

  useEffect(() => () => window.clearTimeout(pending.current), []);

  const update = useCallback(
    (patch: Partial<ControllerProfile>) => {
      setProfile((current) => {
        const next = { ...(current ?? DEFAULT_PROFILE), ...patch };
        window.clearTimeout(pending.current);
        pending.current = window.setTimeout(() => {
          saveControllerProfile(game, next).catch((error) => reportError(i18n.t("errors.save"), error));
        }, SAVE_DELAY_MS);
        return next;
      });
    },
    [game],
  );

  /** For a game: start its own profile from the current one, or go back to the default. */
  const setGameCustom = useCallback(
    (enabled: boolean) => {
      if (!game || !profile) return;
      setCustom(enabled);
      if (enabled) {
        saveControllerProfile(game, profile).catch((error) => reportError(i18n.t("errors.save"), error));
      } else {
        resetControllerProfile(game)
          .then(() => controllerProfile(game))
          .then((loaded) => setProfile(loaded.profile))
          .catch((error) => reportError(i18n.t("errors.save"), error));
      }
    },
    [game, profile],
  );

  return { profile, custom, update, setGameCustom };
}
