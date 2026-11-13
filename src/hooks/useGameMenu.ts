import { useTranslation } from "react-i18next";
import {
  launchWineTool,
  openGameFolder,
  pickProgram,
  runInPrefix,
} from "../lib/backend";
import type { MenuItem } from "../state/contextMenu";
import { useLibraryStore } from "../state/library";
import { reportError } from "../state/toasts";
import { useUi } from "../state/ui";
import type { Game } from "../types/game";
import { useContextMenu } from "./useContextMenu";

/** Right-click menu of a game. Windows games get the Wine tools; Big Picture gets the short version. */
export function useGameMenu(game: Game, onOpen?: () => void) {
  const { t } = useTranslation();
  const { mode, setConfigureGameId, setRemoveGameId } = useUi();
  const store = useLibraryStore;

  return useContextMenu(() => {
    const { runningIds, launchGame, stopGame, toggleFavorite, refreshArtwork } = store.getState();
    const running = runningIds.includes(game.id);
    const fail = (message: string) => (error: unknown) => reportError(message, error);

    const items: MenuItem[] = [
      running
        ? { label: t("game.stop"), icon: "stop", onSelect: () => stopGame(game.id) }
        : { label: t("game.play"), icon: "play", onSelect: () => launchGame(game.id) },
    ];
    if (onOpen) items.push({ label: t("game.details"), icon: "info", onSelect: onOpen });
    items.push({
      label: game.favorite ? t("game.removeFavorite") : t("game.addFavorite"),
      icon: game.favorite ? "starFilled" : "star",
      onSelect: () => toggleFavorite(game.id),
    });
    if (mode === "bigpicture") return items;

    items.push(
      { type: "separator" },
      { label: t("menu.configure"), icon: "settings", onSelect: () => setConfigureGameId(game.id) },
      { label: t("menu.refreshArtwork"), icon: "refresh", onSelect: () => refreshArtwork(game.id) },
    );

    if (game.source === "wine") {
      items.push(
        { type: "separator" },
        {
          label: t("menu.browseFiles"),
          icon: "folder",
          onSelect: () => openGameFolder(game.id, "files").catch(fail(t("errors.folder"))),
        },
        {
          label: t("menu.openPrefix"),
          icon: "folder",
          onSelect: () => openGameFolder(game.id, "prefix").catch(fail(t("errors.folder"))),
        },
        {
          label: t("menu.openLogs"),
          icon: "folder",
          onSelect: () => openGameFolder(game.id, "logs").catch(fail(t("errors.folder"))),
        },
        { type: "separator" },
        {
          label: t("menu.winecfg"),
          icon: "settings",
          onSelect: () => launchWineTool(game.id, "winecfg").catch(fail(t("errors.tool"))),
        },
        {
          label: t("menu.winetricks"),
          icon: "settings",
          onSelect: () => launchWineTool(game.id, "winetricks").catch(fail(t("errors.tool"))),
        },
        {
          label: t("menu.runProgram"),
          icon: "play",
          onSelect: () => {
            const run = async () => {
              const exe = await pickProgram();
              if (exe) await runInPrefix(game.id, exe);
            };
            run().catch(fail(t("errors.tool")));
          },
        },
        { type: "separator" },
        { label: t("game.remove"), icon: "trash", danger: true, onSelect: () => setRemoveGameId(game.id) },
      );
    }
    return items;
  });
}

/** Menu of a page's empty space, so right-clicking anywhere does something sensible. */
export function usePageMenu() {
  const { t } = useTranslation();
  const { setAddGameOpen } = useUi();

  return useContextMenu(() => [
    { label: t("nav.addGame"), icon: "plus", onSelect: () => setAddGameOpen(true) },
    { label: t("menu.rescan"), icon: "refresh", onSelect: () => useLibraryStore.getState().rescan() },
  ]);
}

