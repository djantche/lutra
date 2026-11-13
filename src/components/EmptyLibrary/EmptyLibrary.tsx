import { useTranslation } from "react-i18next";
import { useLibrary } from "../../state/library";
import { useUi } from "../../state/ui";
import { Button } from "../Button/Button";
import { Spinner } from "../Spinner/Spinner";
import { Icon } from "../Icon/Icon";
import styles from "./EmptyLibrary.module.scss";

/** Shown when no source has reported a game yet. `large` scales it up for Big Picture. */
export function EmptyLibrary({ large = false }: { large?: boolean }) {
  const { t } = useTranslation();
  const { scanning, rescan } = useLibrary();
  const { setAddGameOpen } = useUi();

  return (
    <div className={styles.empty} data-large={large || undefined}>
      <span className={styles.icon}>{scanning ? <Spinner /> : <Icon name="gamepad" size={34} />}</span>
      <h2>{scanning ? t("empty.scanning") : t("empty.title")}</h2>
      {!scanning && <p>{t("empty.hint")}</p>}
      {!scanning && (
        <Button variant="primary" size="lg" icon="search" data-autofocus="" onClick={rescan}>
          {t("empty.scan")}
        </Button>
      )}
      {!scanning && !large && (
        <Button variant="ghost" icon="plus" onClick={() => setAddGameOpen(true)}>
          {t("nav.addGame")}
        </Button>
      )}
    </div>
  );
}
