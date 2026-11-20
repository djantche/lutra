import { useTranslation } from "react-i18next";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import { ProgressBar } from "../ProgressBar/ProgressBar";
import styles from "./Stepper.module.scss";

export interface StepperStep {
  id: string;
  label: string;
}

interface StepperProps {
  steps: StepperStep[];
  /** Index of the step in progress. */
  current: number;
}

/**
 * The steps of a flow, with the ones done checked, the current one lit, and a bar that fills as
 * the flow goes on. Labels of the other steps give way on a narrow window.
 */
export function Stepper({ steps, current }: StepperProps) {
  const { t } = useTranslation();
  const label = t("stepper.progress", { current: current + 1, total: steps.length });

  return (
    <div className={styles.stepper}>
      <ol className={styles.steps} aria-label={label}>
        {steps.map((step, index) => {
          const state = index < current ? "done" : index === current ? "current" : "todo";
          return (
            <li key={step.id} className={cx(styles.step, styles[state])} aria-current={state === "current" ? "step" : undefined}>
              <span className={styles.badge}>{state === "done" ? <Icon name="check" size={14} /> : index + 1}</span>
              <span className={styles.label}>{step.label}</span>
            </li>
          );
        })}
      </ol>
      {/* A row, so the bar takes the width and keeps its own height. */}
      <div className={styles.bar}>
        <ProgressBar value={(current + 1) / steps.length} ariaLabel={label} />
      </div>
    </div>
  );
}
