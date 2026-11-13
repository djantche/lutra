import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import styles from "./WindowControls.module.scss";

/** Minimize, maximize and close, for the frameless window. Only rendered inside Tauri. */
export function WindowControls() {
  const { t } = useTranslation();
  const appWindow = getCurrentWindow();

  return (
    <div className={styles.controls}>
      <button type="button" aria-label={t("window.minimize")} onClick={() => void appWindow.minimize()}>
        <Icon name="minimize" size={16} />
      </button>
      <button type="button" aria-label={t("window.maximize")} onClick={() => void appWindow.toggleMaximize()}>
        <Icon name="maximize" size={14} />
      </button>
      <button type="button" className={styles.close} aria-label={t("window.close")} onClick={() => void appWindow.close()}>
        <Icon name="close" size={16} />
      </button>
    </div>
  );
}
