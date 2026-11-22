import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { FormField } from "../../../components/FormField/FormField";
import { Icon } from "../../../components/Icon/Icon";
import { Select } from "../../../components/Select/Select";
import { Spinner } from "../../../components/Spinner/Spinner";
import type { ImportSource, Importable, Runner } from "../../../lib/backend";
import { cx } from "../../../lib/cx";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

const SOURCES: ImportSource[] = ["lutris", "heroic", "bottles"];

/** A game that can be picked: not already here, and its program is still on the disk. */
export const isImportable = (game: Importable) => !game.already && !game.missing;

interface ImportStepProps {
  /** Null while the launchers are being read. */
  games: Importable[] | null;
  selected: Set<string>;
  onSelected: (keys: Set<string>) => void;
  runners: Runner[] | null;
  runnerPath: string;
  onRunner: (path: string) => void;
}

/** Brings games over from Lutris, Heroic and Bottles: pick the ones to keep. */
export function ImportStep({ games, selected, onSelected, runners, runnerPath, onRunner }: ImportStepProps) {
  const { t } = useTranslation();
  const pickable = games?.filter(isImportable) ?? [];
  const toggle = (key: string) => {
    const next = new Set(selected);
    if (!next.delete(key)) next.add(key);
    onSelected(next);
  };
  const allOn = pickable.length > 0 && pickable.every((game) => selected.has(game.key));
  const needsRunner = games?.some((game) => selected.has(game.key) && !game.native) ?? false;

  return (
    <>
      <StepHeading icon="download" title={t("addGame.steps.import.title")} hint={t("addGame.steps.import.hint")} />

      {games === null ? (
        <p className={styles.artHint}>
          <Spinner /> {t("addGame.import.searching")}
        </p>
      ) : games.length === 0 ? (
        <p className={styles.warning}>{t("addGame.import.none")}</p>
      ) : (
        <>
          <div className={styles.importBar}>
            <span>{t("addGame.import.selected", { count: selected.size, total: pickable.length })}</span>
            {pickable.length > 0 && (
              <Button
                variant="ghost"
                onClick={() => onSelected(allOn ? new Set() : new Set(pickable.map((game) => game.key)))}
              >
                {allOn ? t("addGame.import.selectNone") : t("addGame.import.selectAll")}
              </Button>
            )}
          </div>

          {SOURCES.map((source) => {
            const group = games.filter((game) => game.from === source);
            if (group.length === 0) return null;
            return (
              <section key={source} className={styles.section}>
                <h4 className={styles.sectionTitle}>
                  {t(`addGame.import.sources.${source}`)} · {group.length}
                </h4>
                <ul className={styles.importList}>
                  {group.map((game, index) => {
                    const pickableGame = isImportable(game);
                    const on = selected.has(game.key);
                    return (
                      <li key={game.key}>
                        <button
                          type="button"
                          role="checkbox"
                          aria-checked={on}
                          data-focusable
                          data-autofocus={source === games.find(isImportable)?.from && index === group.findIndex(isImportable) ? "" : undefined}
                          disabled={!pickableGame}
                          className={cx(styles.importRow, on && styles.importOn)}
                          onClick={() => toggle(game.key)}
                        >
                          <span className={styles.importBox} aria-hidden="true">
                            {on && <Icon name="check" size={14} />}
                          </span>
                          <span className={styles.importText}>
                            <strong>{game.title}</strong>
                            <small title={game.exePath}>{game.exePath}</small>
                          </span>
                          {game.already ? (
                            <span className={styles.chip}>{t("addGame.import.already")}</span>
                          ) : game.missing ? (
                            <span className={cx(styles.chip, styles.chipWarn)}>{t("addGame.import.missing")}</span>
                          ) : (
                            <>
                              {game.native && <span className={styles.chip}>Linux</span>}
                              {game.prefixPath && <span className={styles.chip}>{t("addGame.import.keepsPrefix")}</span>}
                            </>
                          )}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </section>
            );
          })}

          {needsRunner && runners !== null && runners.length > 0 && (
            <FormField label={t("addGame.import.fallback")} hint={t("addGame.import.fallbackHint")}>
              <Select
                ariaLabel={t("addGame.import.fallback")}
                value={runnerPath}
                onChange={onRunner}
                options={runners.map((runner) => ({ value: runner.path, label: runner.label }))}
              />
            </FormField>
          )}
          {needsRunner && runners !== null && runners.length === 0 && <p className={styles.warning}>{t("addGame.noRunner")}</p>}
        </>
      )}
    </>
  );
}
