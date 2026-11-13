import { useTranslation } from "react-i18next";
import { Select } from "../../../components/Select/Select";
import { Switch } from "../../../components/Switch/Switch";
import styles from "./Tabs.module.scss";

const SIZES = ["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];
const DEFAULT_SIZE = "1920x1080";

interface VirtualDesktopFieldProps {
  value: string | null;
  onChange: (value: string | null) => void;
}

/** On/off plus a resolution; a size typed by hand before stays selectable. */
export function VirtualDesktopField({ value, onChange }: VirtualDesktopFieldProps) {
  const { t } = useTranslation();
  const enabled = Boolean(value);
  const sizes = value && !SIZES.includes(value) ? [value, ...SIZES] : SIZES;

  return (
    <div className={styles.inline}>
      {enabled && (
        <div className={styles.size}>
          <Select
            ariaLabel={t("config.virtualDesktopSize")}
            value={value ?? DEFAULT_SIZE}
            options={sizes.map((size) => ({ value: size, label: size.replace("x", " × ") }))}
            onChange={onChange}
          />
        </div>
      )}
      <Switch
        ariaLabel={t("config.virtualDesktop")}
        checked={enabled}
        onChange={(on) => onChange(on ? (value ?? DEFAULT_SIZE) : null)}
      />
    </div>
  );
}
