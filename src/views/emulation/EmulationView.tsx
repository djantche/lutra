import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Icon } from "../../components/Icon/Icon";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { Spinner } from "../../components/Spinner/Spinner";
import { makerRank } from "../../lib/platforms";
import type { Game } from "../../types/game";
import { SystemPage } from "./SystemPage";
import { SystemTile } from "./SystemTile";
import { useEmulation, type SystemEntry } from "../../hooks/useEmulation";
import styles from "./Emulation.module.scss";

type Filter = "mine" | "ready" | "all";

interface EmulationViewProps {
  /** The system whose page is open, or null for the dashboard. */
  system: string | null;
  onSystem: (name: string | null) => void;
  /** Draws the games of a system the way the shell draws games. */
  renderGames: (games: Game[]) => ReactNode;
  /** Opens the add-a-game window on the console flow (desktop mode only). */
  onAddRom?: (system: string | null) => void;
  /** Opens the emulation settings (folders, BIOS). */
  onOpenSettings: () => void;
}

/**
 * Emulation home, in the spirit of EmulationStation: every console as a tile, grouped by maker,
 * with how many games it holds and whether it can play them; a console opens its own page.
 */
export function EmulationView({ system, onSystem, renderGames, onAddRom, onOpenSettings }: EmulationViewProps) {
  const { t } = useTranslation();
  const emulation = useEmulation();
  const { systems, status, scan, scanning, addRomFolder } = emulation;
  const [chosenFilter, setFilter] = useState<Filter | null>(null);

  const hasGames = systems?.some((entry) => entry.gameCount > 0) ?? false;
  // Until the user picks one: their consoles when they have games, else what can play.
  const filter = chosenFilter ?? (hasGames ? "mine" : "ready");

  const groups = useMemo(() => {
    const visible = (systems ?? []).filter((entry) => {
      if (filter === "mine") return entry.gameCount > 0;
      if (filter === "ready") return entry.gameCount > 0 || (entry.cores.length > 0 && entry.platform.main);
      return true;
    });
    const byMaker = new Map<string, SystemEntry[]>();
    for (const entry of visible) {
      const list = byMaker.get(entry.platform.maker) ?? [];
      list.push(entry);
      byMaker.set(entry.platform.maker, list);
    }
    return [...byMaker.entries()]
      .sort(([a], [b]) => makerRank(a) - makerRank(b) || a.localeCompare(b))
      .map(([maker, list]) => ({
        maker,
        systems: list.sort((a, b) => (a.platform.year ?? 9999) - (b.platform.year ?? 9999)),
      }));
  }, [systems, filter]);

  const open = systems?.find((entry) => entry.name === system);
  if (open) {
    return (
      <SystemPage
        system={open}
        emulation={emulation}
        renderGames={renderGames}
        onAddRom={onAddRom}
        onOpenSettings={onOpenSettings}
      />
    );
  }

  const totalGames = systems?.reduce((sum, entry) => sum + entry.gameCount, 0) ?? 0;
  const readyCount = systems?.filter((entry) => entry.state === "ready" && entry.platform.main).length ?? 0;
  // The emulators Lutra uses: the first core of each console.
  const emulatorCount = new Set(systems?.flatMap((entry) => entry.cores.slice(0, 1).map((core) => core.id))).size;

  return (
    <div className={styles.view}>
      <header className={styles.dashboardHeader}>
        <div className={styles.headline}>
          <h1>{t("emulation.title")}</h1>
          <p>{t("emulation.subtitle")}</p>
        </div>
        <div className={styles.actions}>
          {onAddRom && (
            <Button variant="primary" icon="plus" onClick={() => onAddRom(null)}>
              {t("emulation.addGame")}
            </Button>
          )}
          <Button
            variant="secondary"
            icon={scanning ? undefined : "refresh"}
            disabled={scanning || !status || status.romDirs.length === 0}
            onClick={scan}
          >
            {scanning && <Spinner />}
            {t("emulation.scan")}
          </Button>
          <Button variant="ghost" icon="settings" onClick={onOpenSettings}>
            {t("emulation.settings")}
          </Button>
        </div>
      </header>

      {systems && status && (
        <div className={styles.stats}>
          <div className={styles.stat}>
            <strong>{totalGames}</strong>
            <span>{t("emulation.statGames", { count: totalGames })}</span>
          </div>
          <div className={styles.stat}>
            <strong>{readyCount}</strong>
            <span>{t("emulation.statReady", { count: readyCount })}</span>
          </div>
          <div className={styles.stat}>
            <strong>{status.romDirs.length}</strong>
            <span>{t("emulation.statFolders", { count: status.romDirs.length })}</span>
          </div>
          <div className={styles.stat}>
            <strong>{emulatorCount}</strong>
            <span>{t("emulation.statEmulators", { count: emulatorCount })}</span>
          </div>
        </div>
      )}

      {status && status.romDirs.length === 0 && (
        <div className={styles.notice}>
          <Icon name="folder" size={20} />
          <div>
            <strong>{t("emulation.noFolder")}</strong>
            <p>{t("emulation.noFolderHint")}</p>
          </div>
          <Button variant="primary" icon="plus" onClick={addRomFolder}>
            {t("emulation.addFolder")}
          </Button>
        </div>
      )}

      {systems === null ? (
        <div className={styles.loading}>
          <Spinner />
        </div>
      ) : (
        <>
          <SegmentedControl
            ariaLabel={t("emulation.filter")}
            segments={[
              { value: "mine", label: t("emulation.filterMine") },
              { value: "ready", label: t("emulation.filterReady") },
              { value: "all", label: t("emulation.filterAll") },
            ]}
            value={filter}
            onChange={setFilter}
          />
          {groups.length === 0 ? (
            <div className={styles.empty}>
              <Icon name="gamepad" size={30} />
              <h2>{filter === "mine" ? t("emulation.noGames") : t("emulation.noSystems")}</h2>
              <p>{filter === "mine" ? t("emulation.noGamesHint") : t("emulation.noSystemsHint")}</p>
            </div>
          ) : (
            // Makers sit side by side and wrap, so a maker with one console does not take a row.
            <div className={styles.groups}>
              {groups.map((group, groupIndex) => (
                <section key={group.maker || "other"} className={styles.group}>
                  <h2>{group.maker || t("emulation.otherMakers")}</h2>
                  <div className={styles.grid}>
                    {group.systems.map((entry, index) => (
                      <SystemTile
                        key={entry.name}
                        system={entry}
                        autofocus={groupIndex === 0 && index === 0}
                        onOpen={onSystem}
                      />
                    ))}
                  </div>
                </section>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
