import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Icon, type IconName } from "../../../components/Icon/Icon";
import { NEW_PREFIX } from "../../../components/PrefixPicker/PrefixPicker";
import { listPrefixes, recommendedConfig, type Runner, type RunnerKind, type WineConfig } from "../../../lib/backend";
import { prefixTitle } from "../../../lib/prefixes";
import type { Kind } from "./SourceStep";
import type { Step } from "./steps";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

interface ReviewStepProps {
  kind: Kind;
  title: string;
  developer: string;
  releaseDate: string;
  /** The program that will be added (for an installed game, the one the installer left). */
  program: string;
  runner: Runner | undefined;
  isDefaultRunner: boolean;
  prefixPath: string;
  /** What a new prefix will be called. */
  newPrefixName: string;
  /** Takes the user back to a step to change what it holds. */
  onEdit: (step: Step) => void;
}

interface RowProps {
  icon: IconName;
  label: string;
  value: string;
  detail?: string;
  /** The step that holds this; absent when it can no longer be changed here. */
  step?: Step;
  onEdit: (step: Step) => void;
}

function Row({ icon, label, value, detail, step, onEdit }: RowProps) {
  const { t } = useTranslation();
  return (
    <li className={styles.reviewRow}>
      <span className={styles.reviewIcon} aria-hidden="true">
        <Icon name={icon} size={18} />
      </span>
      <div className={styles.reviewText}>
        <small>{label}</small>
        <strong title={value}>{value}</strong>
        {detail && <small className={styles.reviewDetail}>{detail}</small>}
      </div>
      {step && (
        <Button variant="ghost" icon="edit" aria-label={t("addGame.review.edit", { what: label })} onClick={() => onEdit(step)}>
          {t("addGame.review.change")}
        </Button>
      )}
    </li>
  );
}

/** What the options chosen for this machine switch on, as short labels. */
function featureLabels(config: WineConfig, t: (key: string) => string): string[] {
  const labels: string[] = [];
  if (config.gpu) labels.push(t("addGame.review.features.gpu"));
  if (config.ntsync) labels.push("ntsync");
  else if (config.fsync) labels.push("fsync");
  else if (config.esync) labels.push("esync");
  if (config.gamemode) labels.push("GameMode");
  if (config.nvapi) labels.push("NVAPI");
  if (config.wayland) labels.push("Wayland");
  if (config.fsr) labels.push("FSR");
  if (config.perfTweaks) labels.push(t("addGame.review.features.tweaks"));
  return labels;
}

/** Last check before the game is added: everything that was chosen, each with a way to change it. */
export function ReviewStep({
  kind,
  title,
  developer,
  releaseDate,
  program,
  runner,
  isDefaultRunner,
  prefixPath,
  newPrefixName,
  onEdit,
}: ReviewStepProps) {
  const { t } = useTranslation();
  const [features, setFeatures] = useState<string[] | null>(null);
  const [prefixLabel, setPrefixLabel] = useState<string | null>(null);

  const runnerKind: RunnerKind = kind === "linux" ? "native" : (runner?.kind ?? "wine");

  useEffect(() => {
    let cancelled = false;
    recommendedConfig(runnerKind)
      .then((config) => !cancelled && setFeatures(featureLabels(config, t)))
      .catch(() => !cancelled && setFeatures([]));
    return () => {
      cancelled = true;
    };
  }, [runnerKind, t]);

  const existing = kind !== "linux" && kind !== "install" && prefixPath !== NEW_PREFIX ? prefixPath : null;
  useEffect(() => {
    if (!existing) return;
    let cancelled = false;
    listPrefixes()
      .then((found) => {
        const prefix = found.find((candidate) => candidate.path === existing);
        if (!cancelled) setPrefixLabel(prefix ? prefixTitle(prefix, t) : null);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [existing, t]);

  const facts = [developer.trim(), releaseDate.trim().slice(0, 4)].filter(Boolean).join(" · ");
  const installed = kind === "install";

  return (
    <>
      <StepHeading icon="check" title={t("addGame.steps.review.title")} hint={t("addGame.steps.review.hint")} />

      <ul className={styles.review}>
        <Row icon="gamepad" label={t("addGame.review.game")} value={title} detail={facts || undefined} step="details" onEdit={onEdit} />
        <Row
          icon="folder"
          label={installed ? t("addGame.review.installedProgram") : t("addGame.program")}
          value={program}
          step={installed ? "install" : "source"}
          onEdit={onEdit}
        />
        {kind === "linux" ? (
          <Row icon="terminal" label={t("addGame.review.launcher")} value={t("addGame.review.native")} onEdit={onEdit} />
        ) : (
          <>
            <Row
              icon="settings"
              label={t("addGame.review.launcher")}
              value={runner?.label ?? "—"}
              detail={isDefaultRunner ? t("addGame.runnerDefault") : undefined}
              // The installer already ran with this launcher: changing it now would not move the prefix.
              step={installed ? undefined : "launcher"}
              onEdit={onEdit}
            />
            <Row
              icon="folder"
              label={t("config.prefix")}
              value={
                installed
                  ? t("addGame.review.installPrefix")
                  : existing
                    ? (prefixLabel ?? existing)
                    : t("addGame.review.newPrefix", { name: newPrefixName })
              }
              detail={existing ?? undefined}
              step={installed ? undefined : "launcher"}
              onEdit={onEdit}
            />
          </>
        )}
      </ul>

      <section className={styles.section}>
        <h4 className={styles.sectionTitle}>{t("addGame.review.settings")}</h4>
        {features === null ? null : features.length > 0 ? (
          <div className={styles.chips}>
            {features.map((feature) => (
              <span key={feature} className={styles.chip}>
                <Icon name="check" size={12} />
                {feature}
              </span>
            ))}
          </div>
        ) : null}
        <p className={styles.note}>{t("addGame.review.settingsHint")}</p>
      </section>
    </>
  );
}
