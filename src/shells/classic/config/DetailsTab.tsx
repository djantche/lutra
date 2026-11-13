import { useTranslation } from "react-i18next";
import { GameDetailsFields } from "../../../components/GameDetailsFields/GameDetailsFields";
import { detailsFromMeta, type DetailsForm } from "../../../components/GameDetailsFields/details";
import { GameMatches } from "../../../components/GameMatches/GameMatches";
import { SettingsGroup } from "../../../components/SettingsGroup/SettingsGroup";
import { useGameLookup } from "../../../hooks/useGameLookup";
import type { GameHit } from "../../../lib/backend";
import styles from "./Tabs.module.scss";

interface DetailsTabProps {
  title: string;
  details: DetailsForm;
  onChange: (details: DetailsForm) => void;
}

/** Release date, developer, genres and description, with a lookup in GameDB to fill them in. */
export function DetailsTab({ title, details, onChange }: DetailsTabProps) {
  const { t } = useTranslation();
  const lookup = useGameLookup(title);

  return (
    <>
      <SettingsGroup title={t("details.title")}>
        <div className={styles.block}>
          <GameDetailsFields value={details} onChange={onChange} />
        </div>
      </SettingsGroup>
      <SettingsGroup title={t("details.fromGameDb")}>
        <div className={styles.block}>
          <p className={styles.help}>{t("details.lookupHint", { title })}</p>
          <GameMatches
            {...lookup}
            selected={details.igdbId}
            onSelect={(hit: GameHit) => onChange(detailsFromMeta(hit.meta))}
          />
        </div>
      </SettingsGroup>
    </>
  );
}
