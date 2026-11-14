import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../../components/Button/Button";
import { TextField } from "../../../../components/TextField/TextField";
import { importArtwork, pickImage, type ArtworkKind } from "../../../../lib/backend";
import { useLibraryStore } from "../../../../state/library";
import { reportError } from "../../../../state/toasts";
import type { Game } from "../../../../types/game";
import styles from "./Artwork.module.scss";

/** Lets the user bring their own cover or banner, from a file or a link; the backend keeps a copy. */
export function ArtworkImport({ game, kind }: { game: Game; kind: ArtworkKind }) {
  const { t } = useTranslation();
  const upsertGame = useLibraryStore((state) => state.upsertGame);
  const [source, setSource] = useState("");
  const [busy, setBusy] = useState(false);

  const run = (from: string) => {
    if (busy || from.trim() === "") return;
    setBusy(true);
    importArtwork(game.id, kind, from)
      .then((updated) => {
        upsertGame(updated);
        setSource("");
      })
      .catch((error) => reportError(t("errors.artwork"), error))
      .finally(() => setBusy(false));
  };

  // A file chosen in the native dialog is already a clear decision: no second click needed.
  const browse = () => {
    pickImage()
      .then((path) => path && run(path))
      .catch((error) => reportError(t("errors.artwork"), error));
  };

  return (
    <div className={styles.import}>
      <span className={styles.label}>{t("config.artworkOwn")}</span>
      <div className={styles.importRow}>
        <TextField
          className={styles.importField}
          icon="image"
          clearable
          value={source}
          onChange={setSource}
          onKeyDown={(event) => event.key === "Enter" && run(source)}
          placeholder={t("config.artworkOwnPlaceholder")}
          aria-label={t("config.artworkOwnPlaceholder")}
          spellCheck={false}
          disabled={busy}
        />
        <Button icon="download" disabled={busy || source.trim() === ""} onClick={() => run(source)}>
          {t("config.artworkOwnImport")}
        </Button>
        <Button variant="secondary" icon="folder" disabled={busy} onClick={browse}>
          {t("config.artworkOwnBrowse")}
        </Button>
      </div>
    </div>
  );
}
