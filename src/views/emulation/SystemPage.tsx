import { useState, type CSSProperties, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Icon } from "../../components/Icon/Icon";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../components/Spinner/Spinner";
import { SystemIcon } from "../../components/SystemIcon/SystemIcon";
import { installCore, openEmulationFolder } from "../../lib/backend";
import { useLibraryStore } from "../../state/library";
import { reportError, useToastStore } from "../../state/toasts";
import type { Game } from "../../types/game";
import type { SystemEntry, useEmulation } from "../../hooks/useEmulation";
import { ConsoleSettingsDialog } from "./profile/ConsoleSettingsDialog";
import styles from "./Emulation.module.scss";

interface SystemPageProps {
  system: SystemEntry;
  emulation: ReturnType<typeof useEmulation>;
  renderGames: (games: Game[]) => ReactNode;
  onAddRom?: (system: string | null) => void;
  onOpenSettings: () => void;
}

/** One console: its games, the emulator Lutra runs them with, and the BIOS files it needs. */
export function SystemPage({ system, emulation, renderGames, onAddRom, onOpenSettings }: SystemPageProps) {
  const { t } = useTranslation();
  const notify = useToastStore((state) => state.notify);
  const games = useLibraryStore((state) => state.games);
  const [installing, setInstalling] = useState(false);
  const [configuring, setConfiguring] = useState(false);
  const { platform } = system;
  const own = games.filter((game) => game.system === system.name);
  const missing = system.firmware.filter((file) => !file.optional && file.state === "missing");

  const install = (core: string) => {
    setInstalling(true);
    installCore(core)
      .then((probe) => {
        notify("info", t("emulation.installed", { name: probe.name, version: probe.version }));
        emulation.reload();
      })
      .catch((error) => reportError(t("emulation.installFailed", { name: core }), error))
      .finally(() => setInstalling(false));
  };

  const used = system.cores[0];

  return (
    <div className={styles.view}>
      <header className={styles.systemHero} style={{ "--accent": platform.color } as CSSProperties}>
        <span className={styles.heroWatermark} aria-hidden="true">
          {platform.short}
        </span>
        <SystemIcon platform={platform} icon={system.icon} className={styles.heroIcon} />
        <div className={styles.heroText}>
          <span className={styles.eyebrow}>
            {[platform.maker, platform.year, t(`emulation.kind.${platform.kind}`)].filter(Boolean).join(" · ")}
          </span>
          <h1>{platform.name}</h1>
          <div className={styles.heroChips}>
            <span className={styles.state} data-state={system.state}>
              {t(`emulation.state.${system.state}`)}
            </span>
            <span className={styles.count}>{t("emulation.games", { count: own.length })}</span>
          </div>
        </div>
        <div className={styles.heroActions}>
          {onAddRom && (
            <Button variant="primary" icon="plus" data-autofocus="" onClick={() => onAddRom(system.name)}>
              {t("emulation.addGame")}
            </Button>
          )}
          <Button
            variant="secondary"
            icon="settings"
            data-autofocus={onAddRom ? undefined : ""}
            onClick={() => setConfiguring(true)}
          >
            {t("emulation.profile.configure")}
          </Button>
        </div>
      </header>

      <div className={styles.columns}>
        <section className={styles.gamesColumn}>
          <h2 className={styles.sectionTitle}>{t("emulation.gamesTitle")}</h2>
          {own.length > 0 ? (
            renderGames(own)
          ) : (
            <div className={styles.empty}>
              <Icon name="gamepad" size={30} />
              <h2>{t("emulation.systemEmpty")}</h2>
              <p>{t("emulation.systemEmptyHint", { name: platform.name })}</p>
            </div>
          )}
        </section>

        <aside className={styles.side}>
          <SettingsGroup title={t("emulation.emulator")}>
            {used ? (
              <SettingsRow title={used.name} description={t("emulation.coreByLutra")} />
            ) : (
              <SettingsRow
                title={t("emulation.noCore")}
                description={system.recommendedCore ? t("emulation.noCoreAuto") : t("emulation.noCoreYet")}
              >
                {system.recommendedCore && (
                  <Button
                    variant="secondary"
                    disabled={installing}
                    onClick={() => system.recommendedCore && install(system.recommendedCore)}
                  >
                    {installing && <Spinner />}
                    {t("emulation.installNow")}
                  </Button>
                )}
              </SettingsRow>
            )}
            {system.needsRetroarch && !emulation.status?.retroarch && (
              <SettingsRow title={t("emulation.needsRetroarch")} description={t("emulation.needsRetroarchHint")} />
            )}
            {system.extensions.length > 0 && (
              <SettingsRow
                title={t("emulation.formats")}
                description={system.extensions.map((ext) => `.${ext}`).join("  ")}
              />
            )}
          </SettingsGroup>

          {system.firmware.length > 0 && (
            <SettingsGroup title={t("emulation.bios")}>
              {system.firmware.map((file) => (
                <SettingsRow key={file.path} title={file.path} description={file.description}>
                  <span className={styles.state} data-state={file.state === "present" ? "ready" : file.optional ? "optional" : "bios"}>
                    {t(`emulation.firmware.${file.state}`)}
                    {file.optional && file.state !== "present" ? ` · ${t("emulation.optional")}` : ""}
                  </span>
                </SettingsRow>
              ))}
              <SettingsRow
                stacked
                title={missing.length > 0 ? t("emulation.biosWhere") : t("emulation.biosFolder")}
                description={emulation.status?.biosDir}
              >
                <div className={styles.rowActions}>
                  <Button
                    variant="secondary"
                    icon="folder"
                    onClick={() => openEmulationFolder("bios").catch((error) => reportError(t("errors.folder"), error))}
                  >
                    {t("settings.openFolder")}
                  </Button>
                  <Button variant="ghost" onClick={emulation.reload}>
                    {t("emulation.recheck")}
                  </Button>
                  <Button variant="ghost" onClick={onOpenSettings}>
                    {t("emulation.changeFolder")}
                  </Button>
                </div>
              </SettingsRow>
            </SettingsGroup>
          )}

        </aside>
      </div>

      {configuring && (
        <ConsoleSettingsDialog system={system.name} icon={system.icon} onClose={() => setConfiguring(false)} />
      )}
    </div>
  );
}
