import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import type { Tools } from "../../lib/backend";
import styles from "./ToolsStatus.module.scss";

/** Which helper programs were found, so the user knows what the game options can rely on. */
export function ToolsStatus({ tools }: { tools: Tools }) {
  const { t } = useTranslation();

  const rows: { name: string; found: string | null; detail?: string | null }[] = [
    { name: "Wine", found: tools.wine, detail: tools.wineVersion },
    { name: "umu-launcher", found: tools.umu },
    { name: "Winetricks", found: tools.winetricks },
    { name: "GameMode", found: tools.gamemode },
    { name: "MangoHud", found: tools.mangohud },
  ];

  return (
    <ul className={styles.list}>
      {rows.map((row) => (
        <li key={row.name} className={styles.row} data-found={row.found !== null}>
          <Icon name={row.found ? "check" : "close"} size={14} />
          <span className={styles.name}>{row.name}</span>
          <span className={styles.detail}>{row.found ? (row.detail ?? row.found) : t("config.notInstalled")}</span>
        </li>
      ))}
    </ul>
  );
}
