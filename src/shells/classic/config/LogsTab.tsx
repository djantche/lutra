import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { LogViewer } from "../../../components/LogViewer/LogViewer";
import { SegmentedControl } from "../../../components/SegmentedControl/SegmentedControl";
import { gameLogs, type GameLogKind } from "../../../lib/backend";
import { useLibraryStore } from "../../../state/library";
import styles from "./Tabs.module.scss";

/** The logs of the game that was launched: its output, the previous run, and the runner's own in debug mode. */
export function LogsTab({ gameId }: { gameId: string }) {
  const { t } = useTranslation();
  const running = useLibraryStore((state) => state.runningIds.includes(gameId));
  const [kinds, setKinds] = useState<GameLogKind[]>(["game"]);
  const [kind, setKind] = useState<GameLogKind>("game");

  // The list changes when a run starts (the old log becomes the previous one).
  useEffect(() => {
    gameLogs(gameId)
      .then((available) => setKinds(available.length > 0 ? available : ["game"]))
      .catch(() => setKinds(["game"]));
  }, [gameId, running]);

  return (
    <>
      {kinds.length > 1 && (
        <div className={styles.logKinds}>
          <SegmentedControl
            size="sm"
            ariaLabel={t("logs.kind")}
            segments={kinds.map((value) => ({ value, label: t(`logs.${value}`) }))}
            value={kinds.includes(kind) ? kind : "game"}
            onChange={setKind}
          />
        </div>
      )}
      <LogViewer target={gameId} kind={kinds.includes(kind) ? kind : "game"} live={running && kind === "game"} />
    </>
  );
}
