import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Icon } from "../../components/Icon/Icon";
import { ProgressBar } from "../../components/ProgressBar/ProgressBar";
import { openExternal, type Tools } from "../../lib/backend";
import { cx } from "../../lib/cx";
import {
  TOOL_HOMEPAGES,
  installTool,
  installingTool,
  onToolInstall,
  toolInstallPlans,
  type ToolId,
  type ToolPlan,
} from "../../lib/tools";
import { reportError } from "../../state/toasts";
import styles from "./ToolsStatus.module.scss";

/** What an install said last, and how it ended. */
interface Job {
  tool: ToolId;
  running: boolean;
  line: string;
  /** `no_authentication`, an exit code, or an error from starting it. */
  failure: string | null;
}

/** Which helper programs were found, and a way to install the ones that are missing. */
export function ToolsStatus({ tools, onInstalled }: { tools: Tools; onInstalled: () => void }) {
  const { t } = useTranslation();
  const [plans, setPlans] = useState<ToolPlan[]>([]);
  const [job, setJob] = useState<Job | null>(null);

  useEffect(() => {
    toolInstallPlans()
      .then(setPlans)
      .catch(() => setPlans([]));
    // The page may open in the middle of an install.
    void installingTool().then((tool) => tool && setJob((now) => now ?? { tool, running: true, line: "", failure: null }));
    const stop = onToolInstall((event) => {
      if (event.state === "running") {
        setJob({ tool: event.tool, running: true, line: event.line ?? "", failure: null });
      } else if (event.state === "done") {
        setJob(null);
        onInstalled();
      } else {
        setJob(event.state === "cancelled" ? null : { tool: event.tool, running: false, line: "", failure: event.message ?? "" });
      }
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [onInstalled]);

  const rows: { tool: ToolId; name: string; found: string | null; detail?: string | null }[] = [
    { tool: "wine", name: "Wine", found: tools.wine, detail: tools.wineVersion },
    { tool: "umu", name: "umu-launcher", found: tools.umu },
    { tool: "winetricks", name: "Winetricks", found: tools.winetricks },
    { tool: "gamemode", name: "GameMode", found: tools.gamemode },
    { tool: "mangohud", name: "MangoHud", found: tools.mangohud },
  ];

  const install = (tool: ToolId) => {
    setJob({ tool, running: true, line: "", failure: null });
    installTool(tool).catch((error) => {
      setJob(null);
      reportError(t("tools.installError"), error);
    });
  };

  const busy = job?.running ?? false;

  return (
    <ul className={styles.list}>
      {rows.map((row) => {
        const plan = plans.find((candidate) => candidate.tool === row.tool);
        const mine = job?.tool === row.tool ? job : null;
        return (
          <li key={row.tool} className={styles.item}>
            <div className={styles.row} data-found={row.found !== null}>
              <Icon name={row.found ? "check" : "close"} size={14} />
              <span className={styles.name}>{row.name}</span>
              <span className={styles.detail}>{row.found ? (row.detail ?? row.found) : t("config.notInstalled")}</span>
              {row.found === null && !mine?.running && (
                <MissingAction tool={row.tool} plan={plan} busy={busy} onInstall={() => install(row.tool)} />
              )}
            </div>
            {mine?.running && (
              <div className={styles.progress}>
                <ProgressBar value={null} ariaLabel={t("tools.installing", { tool: row.name })} />
                <span className={styles.line}>{mine.line || t("tools.waiting")}</span>
              </div>
            )}
            {mine && !mine.running && mine.failure !== null && (
              <p className={cx(styles.progress, styles.failure)}>
                {mine.failure === "no_authentication" ? t("tools.noAuthentication") : t("tools.failed", { reason: mine.failure })}
                {plan?.command && <CopyCommand command={plan.command} />}
              </p>
            )}
          </li>
        );
      })}
    </ul>
  );
}

/** What can be done about a missing tool: install it, copy the command, or go and get it. */
function MissingAction({ tool, plan, busy, onInstall }: { tool: ToolId; plan?: ToolPlan; busy: boolean; onInstall: () => void }) {
  const { t } = useTranslation();
  if (plan?.canRun) {
    return (
      <Button variant="secondary" icon="download" className={styles.action} disabled={busy} onClick={onInstall}>
        {t("tools.install")}
      </Button>
    );
  }
  if (plan?.command) return <CopyCommand command={plan.command} />;
  const homepage = TOOL_HOMEPAGES[tool];
  return homepage ? (
    <Button variant="ghost" icon="external" className={styles.action} onClick={() => void openExternal(homepage)}>
      {t("tools.get")}
    </Button>
  ) : null;
}

function CopyCommand({ command }: { command: string }) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  return (
    <Button
      variant="ghost"
      icon={copied ? "check" : "copy"}
      className={styles.action}
      title={command}
      onClick={() =>
        navigator.clipboard
          .writeText(command)
          .then(() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1800);
          })
          .catch((error) => reportError(t("errors.copy"), error))
      }
    >
      {copied ? t("tools.copied") : t("tools.copyCommand")}
    </Button>
  );
}
