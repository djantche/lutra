import { useTranslation } from "react-i18next";
import { FormField } from "../FormField/FormField";
import { TextArea } from "../TextArea/TextArea";
import { TextField } from "../TextField/TextField";
import { isValidDate, type DetailsForm } from "./details";
import styles from "./GameDetailsFields.module.scss";

interface GameDetailsFieldsProps {
  value: DetailsForm;
  onChange: (value: DetailsForm) => void;
}

/** Release date, developer, genres and description: what GameDB fills in, all editable. */
export function GameDetailsFields({ value, onChange }: GameDetailsFieldsProps) {
  const { t } = useTranslation();
  const change = (patch: Partial<DetailsForm>) => onChange({ ...value, ...patch });
  const dateValid = isValidDate(value.releaseDate);

  return (
    <div className={styles.grid}>
      <FormField label={t("details.releaseDate")} error={dateValid ? null : t("details.dateInvalid")}>
        <TextField
          value={value.releaseDate}
          placeholder="2018-09-28"
          invalid={!dateValid}
          inputMode="numeric"
          onChange={(releaseDate) => change({ releaseDate })}
        />
      </FormField>
      <FormField label={t("details.developer")}>
        <TextField value={value.developer} onChange={(developer) => change({ developer })} />
      </FormField>
      <div className={styles.wide}>
        <FormField label={t("details.genres")} hint={t("details.genresHint")}>
          <TextField value={value.genres} placeholder={t("details.genresPlaceholder")} onChange={(genres) => change({ genres })} />
        </FormField>
      </div>
      <div className={styles.wide}>
        <FormField label={t("details.description")}>
          <TextArea value={value.description} onChange={(description) => change({ description })} />
        </FormField>
      </div>
    </div>
  );
}
