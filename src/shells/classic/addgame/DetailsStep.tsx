import { useTranslation } from "react-i18next";
import { FormField } from "../../../components/FormField/FormField";
import { GameDetailsFields } from "../../../components/GameDetailsFields/GameDetailsFields";
import type { DetailsForm } from "../../../components/GameDetailsFields/details";
import { GameMatches } from "../../../components/GameMatches/GameMatches";
import { TextField } from "../../../components/TextField/TextField";
import type { GameLookup } from "../../../hooks/useGameLookup";
import type { GameHit } from "../../../lib/backend";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

interface DetailsStepProps {
  title: string;
  onTitle: (title: string) => void;
  details: DetailsForm;
  onDetails: (details: DetailsForm) => void;
  lookup: GameLookup;
  matchId: number | null;
  onMatch: (hit: GameHit) => void;
}

/** The game's name and details: GameDB's guess for what was typed, all of it editable. */
export function DetailsStep({ title, onTitle, details, onDetails, lookup, matchId, onMatch }: DetailsStepProps) {
  const { t } = useTranslation();

  return (
    <>
      <StepHeading icon="info" title={t("addGame.steps.details.title")} hint={t("addGame.steps.details.hint")} />

      <FormField label={t("addGame.name")}>
        <TextField value={title} placeholder={t("addGame.namePlaceholder")} onChange={onTitle} data-autofocus="" />
      </FormField>

      <div className={styles.section}>
        <span className={styles.sectionTitle}>{t("details.fromGameDb")}</span>
        <GameMatches {...lookup} selected={matchId} onSelect={onMatch} />
      </div>

      <GameDetailsFields value={details} onChange={onDetails} />
    </>
  );
}
