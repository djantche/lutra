import { Fragment, useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useAutofocus } from "../../input/useAutofocus";
import { useInputAction } from "../../input/useInputAction";
import { openExternal } from "../../lib/backend";
import { formatBytes } from "../../lib/format";
import { relaunchApp } from "../../lib/host";
import { cancelUpdate, setSkippedVersion } from "../../lib/updates";
import { reportError } from "../../state/toasts";
import { useUpdates } from "../../state/updates";
import { Button } from "../Button/Button";
import { ProgressBar } from "../ProgressBar/ProgressBar";
import styles from "./UpdateScreen.module.scss";

/** `**bold**` and `[text](url)` of a line of release notes. */
function inline(text: string): ReactNode[] {
  return text
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/`/g, "")
    .split(/(\*\*[^*]+\*\*)/g)
    .map((part, index) =>
      part.startsWith("**") && part.endsWith("**") ? <strong key={index}>{part.slice(2, -2)}</strong> : <Fragment key={index}>{part}</Fragment>,
    );
}

/** The release notes (GitHub's Markdown): headings, bullet lists and paragraphs. */
function ReleaseNotes({ markdown }: { markdown: string }) {
  const blocks: ReactNode[] = [];
  let bullets: string[] = [];
  const flush = () => {
    if (bullets.length === 0) return;
    const items = bullets;
    bullets = [];
    blocks.push(
      <ul key={blocks.length}>
        {items.map((item, index) => (
          <li key={index}>{inline(item)}</li>
        ))}
      </ul>,
    );
  };
  for (const raw of markdown.replace(/\r/g, "").split("\n")) {
    const line = raw.trim();
    const bullet = /^[-*]\s+(.*)$/.exec(line);
    const heading = /^#{1,6}\s+(.*)$/.exec(line);
    if (bullet) bullets.push(bullet[1]);
    else if (heading) {
      flush();
      blocks.push(<h3 key={blocks.length}>{inline(heading[1])}</h3>);
    } else if (line !== "") {
      flush();
      blocks.push(<p key={blocks.length}>{inline(line)}</p>);
    } else flush();
  }
  flush();
  return <>{blocks}</>;
}

/**
 * A new release was found when Lutra started: a full-screen page with what changed, and the choice
 * to install it now or later. Installing downloads, checks, then restarts Lutra by itself.
 */
export function UpdateScreen() {
  const { t, i18n } = useTranslation();
  const { info, install, installed, failure, startInstall, closePrompt } = useUpdates();
  const ref = useAutofocus<HTMLDivElement>();

  const later = () => {
    if (info) setSkippedVersion(info.latest);
    closePrompt();
  };
  useInputAction((action) => {
    if (action === "back" && !install && !installed) {
      later();
      return true;
    }
  });

  // The page says Lutra opens again by itself.
  useEffect(() => {
    if (installed) relaunchApp().catch((error) => reportError(t("updates.restartError"), error));
  }, [installed, t]);

  if (!info) return null;
  const size = (bytes: number) => formatBytes(bytes, i18n.language);
  const progress = install && install.total > 0 && install.state === "download" ? install.received / install.total : null;

  return (
    <div className={styles.screen}>
      <div ref={ref} className={styles.content} role="dialog" aria-modal="true" aria-label={t("updates.screen.title", { version: info.latest })} data-nav-trap>
        <img src="/lutra.svg" alt="" width={72} height={72} />
        <h1>{t("updates.screen.title", { version: info.latest })}</h1>
        <p className={styles.subtitle}>
          {info.canInstall
            ? t("updates.screen.body", { current: info.current })
            : t("updates.screen.manual", { current: info.current })}
        </p>

        {info.notes.trim() !== "" && (
          <div className={styles.notes}>
            <ReleaseNotes markdown={info.notes} />
          </div>
        )}

        {installed ? (
          <p className={styles.status}>{t("updates.screen.restarting")}</p>
        ) : install ? (
          <div className={styles.progress}>
            <ProgressBar value={progress} ariaLabel={t("updates.downloading")} />
            <div className={styles.row}>
              <span className={styles.status}>
                {install.state === "download"
                  ? `${t("updates.downloading")} · ${size(install.received)} / ${size(install.total)}`
                  : t(`updates.stage.${install.state}`)}
              </span>
              {install.state === "download" && (
                <Button variant="ghost" icon="close" data-autofocus onClick={() => cancelUpdate().catch((e) => reportError(t("updates.cancelError"), e))}>
                  {t("updates.cancel")}
                </Button>
              )}
            </div>
          </div>
        ) : (
          <>
            {failure !== null && <p className={styles.failure}>{t("updates.failed", { reason: failure })}</p>}
            <div className={styles.actions}>
              <Button size="lg" onClick={later}>
                {t("updates.screen.later")}
              </Button>
              {info.canInstall ? (
                <Button size="lg" variant="primary" data-autofocus onClick={() => void startInstall()}>
                  {t("updates.screen.now")}
                </Button>
              ) : (
                <Button
                  size="lg"
                  variant="primary"
                  icon="external"
                  data-autofocus
                  onClick={() => {
                    void openExternal(info.page);
                    later();
                  }}
                >
                  {t("updates.openPage")}
                </Button>
              )}
            </div>
            <p className={styles.hint}>{t("updates.screen.laterHint")}</p>
          </>
        )}
      </div>
    </div>
  );
}
