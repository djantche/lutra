import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Icon } from "../../../components/Icon/Icon";
import { ProgressBar } from "../../../components/ProgressBar/ProgressBar";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import {
  installComponent,
  listComponents,
  onComponentInstall,
  removeComponent,
  setComponentEnabled,
  type ComponentInfo,
  type ComponentOperation,
} from "../../../lib/components";
import { useLibraryStore } from "../../../state/library";
import { reportError } from "../../../state/toasts";
import styles from "./ComponentsGroup.module.scss";

/**
 * Windows libraries games often need (shader compiler, Visual C++, fonts...), installed in the
 * game's prefix, switched on and off, and taken out again. The prefix is in use while the game
 * runs, so nothing changes then.
 */
export function ComponentsGroup({ gameId }: { gameId: string }) {
  const { t } = useTranslation();
  const gameRunning = useLibraryStore((state) => state.runningIds.includes(gameId));
  const [items, setItems] = useState<ComponentInfo[]>([]);

  const refresh = useCallback(
    () =>
      listComponents(gameId)
        .then(setItems)
        .catch((error) => reportError(t("errors.components"), error)),
    [gameId, t],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // The work runs in the backend and outlives this window: follow it from its events.
  useEffect(() => {
    let cancelled = false;
    let stop = () => {};
    onComponentInstall((event) => {
      if (event.id !== gameId) return;
      if (event.state === "running") {
        // Many of these come while a file downloads: update the row without asking the backend.
        setItems((current) =>
          current.map((item) =>
            item.verb === event.verb
              ? { ...item, operation: event.operation, phase: event.phase, percent: event.percent }
              : item,
          ),
        );
        return;
      }
      if (event.state === "failed") reportError(t("errors.component"), event.message ?? event.verb);
      void refresh();
    }).then((unlisten) => {
      if (cancelled) unlisten();
      else stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop();
    };
  }, [gameId, refresh, t]);

  /** Shows the row as busy at once, then asks the backend. A refusal puts the list back. */
  const run = (verb: string, operation: ComponentOperation, start: () => Promise<void>) => {
    setItems((current) =>
      current.map((item) =>
        item.verb === verb ? { ...item, operation, phase: "install", percent: null } : item,
      ),
    );
    start().catch((error) => {
      reportError(t("errors.component"), error);
      void refresh();
    });
  };

  if (items.length === 0) return null;
  const busy = items.some((item) => item.operation !== null);

  return (
    <SettingsGroup title={t("components.title")}>
      {gameRunning && <SettingsRow title={t("components.gameRunning")} />}
      {items.map((item) => (
        <SettingsRow
          key={item.verb}
          title={t(`components.names.${item.verb}`, item.verb)}
          description={item.outdated ? t("components.outdated") : t(`components.hints.${item.verb}`, "")}
        >
          <ComponentState
            item={item}
            disabled={gameRunning || busy}
            onInstall={() => run(item.verb, "install", () => installComponent(gameId, item.verb))}
            onToggle={(enabled) =>
              run(item.verb, enabled ? "enable" : "disable", () =>
                setComponentEnabled(gameId, item.verb, enabled),
              )
            }
            onRemove={() => run(item.verb, "remove", () => removeComponent(gameId, item.verb))}
          />
        </SettingsRow>
      ))}
    </SettingsGroup>
  );
}

interface ComponentStateProps {
  item: ComponentInfo;
  disabled: boolean;
  onInstall: () => void;
  onToggle: (enabled: boolean) => void;
  onRemove: () => void;
}

/**
 * Where a component stands: a button to install it, a progress while something is done to it, or
 * its switch and a remove button once it is there. Each state fades in, so the row never jumps.
 */
function ComponentState({ item, disabled, onInstall, onToggle, onRemove }: ComponentStateProps) {
  const { t } = useTranslation();

  if (item.operation) return <OperationProgress item={item} operation={item.operation} />;

  if (!item.installed) {
    return (
      <div key="install" className={styles.state}>
        <Button icon="download" disabled={disabled} onClick={onInstall}>
          {t("components.install")}
        </Button>
      </div>
    );
  }

  return (
    <div key="installed" className={styles.state}>
      <div className={styles.controls}>
        {item.outdated ? (
          <Button variant="primary" icon="refresh" disabled={disabled} onClick={onInstall}>
            {t("components.update")}
          </Button>
        ) : item.canToggle ? (
          <label className={styles.toggle}>
            <span className={item.enabled ? styles.on : styles.off}>
              {item.enabled ? t("components.enabled") : t("components.disabled")}
            </span>
            <Switch
              ariaLabel={t("components.switch", { name: t(`components.names.${item.verb}`, item.verb) })}
              checked={item.enabled}
              disabled={disabled}
              onChange={onToggle}
            />
          </label>
        ) : (
          <span className={styles.installed}>
            <Icon name="check" size={16} />
            {t("components.installed")}
          </span>
        )}
        {item.canRemove && (
          <Button
            variant="ghost"
            icon="trash"
            disabled={disabled}
            aria-label={t("components.remove")}
            title={t("components.remove")}
            onClick={onRemove}
          />
        )}
      </div>
    </div>
  );
}

const OPERATION_LABELS: Record<ComponentOperation, string> = {
  install: "components.installing",
  enable: "components.enabling",
  disable: "components.disabling",
  remove: "components.removing",
};

/**
 * A download fills a bar and shows its percentage. Everything else (unpacking, registering,
 * switching, removing) cannot be measured, and neither can a download when `curl` is missing:
 * those only get their name and a bar that slides until it is done.
 */
function OperationProgress({ item, operation }: { item: ComponentInfo; operation: ComponentOperation }) {
  const { t } = useTranslation();
  const downloading = operation === "install" && item.phase === "download";
  const percent = downloading ? item.percent : null;
  const label = downloading ? t("components.downloading") : t(OPERATION_LABELS[operation]);

  return (
    <div key="progress" className={styles.progress} role="status">
      <div className={styles.line}>
        <span className={styles.label}>{label}</span>
        {percent !== null && <span className={styles.percent}>{t("components.percent", { percent })}</span>}
      </div>
      <ProgressBar value={percent === null ? null : percent / 100} ariaLabel={label} />
    </div>
  );
}
