import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { onRunnerInstall, runningRunnerInstalls } from "../lib/runners";
import { playSound } from "../lib/sounds";
import { useRunnerInstalls } from "./runnerInstalls";
import { useToastStore } from "./toasts";

/**
 * Follows the runner installs for the whole app, so one keeps going (and keeps its log) while the
 * user is on another page, and tells them how it ended.
 */
export function RunnerInstallEffects() {
  const { t } = useTranslation();

  useEffect(() => {
    void runningRunnerInstalls()
      .then((names) => names.forEach((name) => useRunnerInstalls.getState().adopt(name)))
      .catch(() => {});
    const stop = onRunnerInstall((event) => {
      useRunnerInstalls.getState().apply(event);
      if (event.stage === "done") {
        playSound("confirm");
        useToastStore.getState().notify("info", t("runners.installed", { name: event.name }));
      } else if (event.stage === "failed") {
        playSound("error");
        useToastStore.getState().notify("error", t("runners.failed", { name: event.name, reason: event.line ?? "" }));
      }
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [t]);

  return null;
}
