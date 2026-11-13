import type { TFunction } from "i18next";
import type { LaunchStage } from "../state/library";

/** What the launch is doing right now, in words. */
export const launchLabel = (t: TFunction, launch: LaunchStage | null): string =>
  t(`launch.stage.${launch?.stage ?? "check"}`);
