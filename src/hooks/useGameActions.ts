import { useTranslation } from "react-i18next";
import { useLibraryStore } from "../state/library";

/** Play/stop state of a game, for every button that starts one. */
export function useGameActions(id: string) {
  const { t } = useTranslation();
  const running = useLibraryStore((state) => state.runningIds.includes(id));
  // The real stage of this game's launch, or null when it is not starting.
  const launch = useLibraryStore((state) => (state.launch?.id === id ? state.launch : null));
  const launchGame = useLibraryStore((state) => state.launchGame);
  const stopGame = useLibraryStore((state) => state.stopGame);

  return {
    running,
    launching: launch !== null,
    launch,
    label: running ? t("game.stop") : t("game.play"),
    /** Starts the game, or stops it if it is already running. */
    toggle: () => (running ? stopGame(id) : launchGame(id)),
  };
}
