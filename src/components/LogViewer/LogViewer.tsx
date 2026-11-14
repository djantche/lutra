import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { clearLog, openLogsFolder, readLog, type GameLogKind, type LogContent } from "../../lib/backend";
import { cx } from "../../lib/cx";
import { reportError } from "../../state/toasts";
import { Button } from "../Button/Button";
import { TextField } from "../TextField/TextField";
import styles from "./LogViewer.module.scss";

const REFRESH_MS = 1500;

/** Colour a line by what it says: Lutra's levels, Wine's `err:`/`warn:`/`fixme:` and the launch header. */
function lineKind(line: string): string | undefined {
  if (/\bERROR\b|(^|:)err:/.test(line)) return styles.error;
  if (/\bWARN\b|(^|:)warn:/.test(line)) return styles.warn;
  if (/(^|:)fixme:/.test(line)) return styles.fixme;
  if (/^(===|Command:|Folder:|Env:)/.test(line)) return styles.meta;
  return undefined;
}

interface LogViewerProps {
  /** `app` for Lutra's own log, or a game id for its launch log. */
  target: string;
  /** Which of the game's logs to show; ignored for Lutra's own. */
  kind?: GameLogKind;
  /** Reloads regularly, e.g. while the game is running. */
  live?: boolean;
}

export function LogViewer({ target, kind = "game", live = false }: LogViewerProps) {
  const { t } = useTranslation();
  const [log, setLog] = useState<LogContent | null>(null);
  const [filter, setFilter] = useState("");
  const scroller = useRef<HTMLPreElement>(null);
  // Stays pinned to the newest lines unless the user scrolled up to read.
  const following = useRef(true);

  const load = useCallback(() => {
    readLog(target, kind)
      .then(setLog)
      .catch((error) => reportError(t("errors.log"), error));
  }, [target, kind, t]);

  useEffect(() => {
    load();
    if (!live) return;
    const timer = window.setInterval(load, REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [load, live]);

  const lines = useMemo(() => {
    const all = (log?.text ?? "").split("\n");
    if (all[all.length - 1] === "") all.pop();
    const needle = filter.trim().toLowerCase();
    return needle ? all.filter((line) => line.toLowerCase().includes(needle)) : all;
  }, [log, filter]);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (element && following.current) element.scrollTop = element.scrollHeight;
  }, [lines]);

  const copy = () => {
    navigator.clipboard
      .writeText(lines.join("\n"))
      .catch((error) => reportError(t("errors.copy"), error));
  };

  const clear = () => {
    clearLog(target, kind)
      .then(load)
      .catch((error) => reportError(t("errors.clearLog"), error));
  };

  return (
    <div className={styles.viewer}>
      <div className={styles.toolbar}>
        <TextField
          className={styles.filter}
          icon="search"
          clearable
          value={filter}
          onChange={setFilter}
          placeholder={t("logs.filter")}
          aria-label={t("logs.filter")}
          spellCheck={false}
        />
        <Button variant="ghost" icon="refresh" aria-label={t("logs.refresh")} title={t("logs.refresh")} onClick={load} />
        <Button variant="ghost" icon="copy" aria-label={t("logs.copy")} title={t("logs.copy")} onClick={copy} />
        <Button
          variant="ghost"
          icon="trash"
          aria-label={t("logs.clear")}
          title={t("logs.clear")}
          disabled={log?.text === ""}
          onClick={clear}
        />
        <Button
          variant="ghost"
          icon="folder"
          aria-label={t("logs.openFolder")}
          title={t("logs.openFolder")}
          onClick={() => openLogsFolder().catch((error) => reportError(t("errors.folder"), error))}
        />
      </div>

      <pre
        ref={scroller}
        className={styles.log}
        tabIndex={0}
        onScroll={(event) => {
          const element = event.currentTarget;
          following.current = element.scrollHeight - element.scrollTop - element.clientHeight < 24;
        }}
      >
        {log?.truncated && <span className={styles.note}>{t("logs.truncated")}{"\n"}</span>}
        {lines.length === 0 ? (
          <span className={styles.note}>{log === null ? "…" : filter ? t("logs.noMatch") : t("logs.empty")}</span>
        ) : (
          lines.map((line, index) => (
            <span key={index} className={cx(lineKind(line))}>
              {line}
              {"\n"}
            </span>
          ))
        )}
      </pre>

      <p className={styles.footer}>
        <span>{t("logs.lines", { count: lines.length })}</span>
        {live && <span className={styles.live}>{t("logs.live")}</span>}
        <span className={styles.path}>{log?.path}</span>
      </p>
    </div>
  );
}
