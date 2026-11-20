import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { hasBackend } from "../lib/backend";
import {
  autoCheck,
  isMajorUpdate,
  lastChecked,
  majorOnly,
  notifiedVersion,
  notifyUpdates,
  onUpdateInstall,
  setNotifiedVersion,
  skippedVersion,
} from "../lib/updates";
import { playSound } from "../lib/sounds";
import { useToastStore } from "./toasts";
import { useUpdates } from "./updates";

const DAY = 24 * 60 * 60 * 1000;
const RECHECK_EVERY = 6 * 60 * 60 * 1000;

/**
 * Keeps the update state alive for the whole session: follows a download's events, looks for a
 * new version as soon as Lutra starts (a full-screen page when there is one) and about once a
 * day after that, announcing each new version once (a message in the app and a system
 * notification, which also reaches the tray).
 */
export function UpdateEffects() {
  const { t } = useTranslation();

  useEffect(() => {
    const stop = onUpdateInstall((event) => {
      const store = useUpdates.getState();
      if (event.state === "done") store.finish(event.message ?? "", null);
      else if (event.state === "failed") store.finish(null, event.message ?? "");
      else if (event.state === "cancelled") store.finish(null, null);
      else
        store.setProgress({ state: event.state, received: event.received ?? 0, total: event.total ?? 0 });
    });
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  useEffect(() => {
    const maybeCheck = async (atStart: boolean) => {
      // The browser preview has no releases to look at.
      if (!hasBackend() || !autoCheck()) return;
      const last = lastChecked();
      if (!atStart && last !== null && Date.now() - last < DAY) return;
      const info = await useUpdates.getState().check();
      if (!info?.available || !notifyUpdates()) return;
      // Smaller releases wait in Settings > About until the user looks.
      if (majorOnly() && !isMajorUpdate(info.current, info.latest)) return;
      if (skippedVersion() === info.latest) return;
      // At start, a new release gets a full-screen page of its own, each time Lutra opens until
      // the user answers "later".
      if (atStart) {
        setNotifiedVersion(info.latest);
        useUpdates.getState().openPrompt();
        playSound("open");
        return;
      }
      if (notifiedVersion() === info.latest) return;
      setNotifiedVersion(info.latest);
      const title = t("updates.availableTitle");
      const body = t("updates.availableBody", { version: info.latest });
      useToastStore.getState().notify("info", `${title} ${body}`);
      playSound("open");
      try {
        new Notification(title, { body, tag: "lutra-update" });
      } catch {
        // The message in the app is enough where system notifications are not available.
      }
    };
    void maybeCheck(true);
    const again = window.setInterval(() => void maybeCheck(false), RECHECK_EVERY);
    return () => window.clearInterval(again);
  }, [t]);

  return null;
}
