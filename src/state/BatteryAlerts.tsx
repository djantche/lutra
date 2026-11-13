import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { onControllerBattery } from "../lib/controllers";
import { playSound } from "../lib/sounds";
import { useToastStore } from "./toasts";

/**
 * Warns when a controller's battery runs low, at 30 % and again at 15 %: a message in the app
 * and a system notification, which also reaches the user while the window is in the tray.
 */
export function BatteryAlerts() {
  const { t } = useTranslation();

  useEffect(() => {
    const stop = onControllerBattery(({ name, percent, level }) => {
      const critical = level <= 15;
      const title = t(critical ? "controllers.battery.criticalTitle" : "controllers.battery.lowTitle");
      const body = t("controllers.battery.alert", { name, percent });
      useToastStore.getState().notify(critical ? "error" : "info", `${title} ${body}`);
      playSound(critical ? "error" : "open");
      try {
        new Notification(title, { body, tag: `battery-${name}` });
      } catch {
        // The toast is enough where system notifications are not available.
      }
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [t]);

  return null;
}
