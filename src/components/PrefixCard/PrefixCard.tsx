import { useTranslation } from "react-i18next";
import type { PrefixInfo } from "../../lib/backend";
import { prefixTitle } from "../../lib/prefixes";
import { Button } from "../Button/Button";
import { Icon } from "../Icon/Icon";
import styles from "./PrefixCard.module.scss";

/** The games that share a prefix, as chips. */
export function GameChips({ games }: { games: string[] }) {
  return (
    <ul className={styles.chips}>
      {games.map((game) => (
        <li key={game}>{game}</li>
      ))}
    </ul>
  );
}

/** What a prefix is: its name, what made it, and the games that live in it. */
export function PrefixCard({ prefix, onRename }: { prefix: PrefixInfo; onRename: () => void }) {
  const { t } = useTranslation();
  const details = [
    t(`prefix.origin.${prefix.origin}`),
    prefix.kind && t(`prefix.kind.${prefix.kind}`),
    prefix.arch === "win32" ? "32-bit" : prefix.arch === "win64" ? "64-bit" : null,
  ].filter(Boolean);

  return (
    <div className={styles.card}>
      <div className={styles.icon} aria-hidden="true">
        <Icon name="folder" size={20} />
      </div>
      <div className={styles.text}>
        <strong>{prefixTitle(prefix, t)}</strong>
        <small>{details.join(" · ")}</small>
        {prefix.games.length > 0 ? <GameChips games={prefix.games} /> : <small>{t("prefix.noGames")}</small>}
      </div>
      {prefix.origin === "lutra" && (
        <Button variant="ghost" icon="edit" onClick={onRename} aria-label={t("prefix.rename")} title={t("prefix.rename")} />
      )}
    </div>
  );
}
