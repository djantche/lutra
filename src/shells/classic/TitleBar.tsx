import { isTauri } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { TextField } from "../../components/TextField/TextField";
import { cx } from "../../lib/cx";
import styles from "./TitleBar.module.scss";
import { WindowControls } from "./WindowControls";

interface TitleBarProps {
  scrolled: boolean;
  canGoBack: boolean;
  onBack: () => void;
  query: string;
  onQuery: (query: string) => void;
}

export function TitleBar({ scrolled, canGoBack, onBack, query, onQuery }: TitleBarProps) {
  const { t } = useTranslation();

  return (
    <header className={cx(styles.bar, scrolled && styles.scrolled)} data-tauri-drag-region>
      <button
        type="button"
        data-focusable
        className={styles.back}
        disabled={!canGoBack}
        aria-label={t("common.back")}
        title={t("common.back")}
        onClick={onBack}
      >
        <Icon name="arrowLeft" size={18} />
      </button>

      <TextField
        className={styles.search}
        icon="search"
        clearable
        value={query}
        onChange={onQuery}
        placeholder={t("search.placeholder")}
        aria-label={t("search.placeholder")}
        spellCheck={false}
      />

      <div className={styles.spacer} data-tauri-drag-region />
      {isTauri() && <WindowControls />}
    </header>
  );
}
