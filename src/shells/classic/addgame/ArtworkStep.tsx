import { useTranslation } from "react-i18next";
import { Spinner } from "../../../components/Spinner/Spinner";
import type { Game } from "../../../types/game";
import { ArtworkTab } from "../config/artwork/ArtworkTab";
import { StepHeading } from "./StepHeading";
import styles from "./AddGame.module.scss";

/**
 * Last step, once the game is in the library: its cover, banner and logo. Lutra already looked for
 * them on its own; this is where to pick others from SteamGridDB, or bring your own.
 */
export function ArtworkStep({ game }: { game: Game | null }) {
  const { t } = useTranslation();

  return (
    <>
      <StepHeading icon="image" title={t("addGame.steps.artwork.title")} hint={t("addGame.steps.artwork.hint")} />
      {game ? (
        <ArtworkTab game={game} />
      ) : (
        <p className={styles.artHint}>
          <Spinner /> {t("addGame.adding")}
        </p>
      )}
    </>
  );
}
