import { useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Slider } from "../../components/Slider/Slider";
import { Switch } from "../../components/Switch/Switch";
import {
  SCALE_MAX,
  SCALE_MIN,
  highContrast,
  reducedMotion,
  setHighContrast,
  setReducedMotion,
  setUiScale,
  uiScale,
} from "../../lib/accessibility";
import styles from "./AccessibilitySection.module.scss";

/** Reading and moving: the size of the interface, contrast, and animations. */
export function AccessibilitySection() {
  const { t } = useTranslation();
  const [scale, setScale] = useState(uiScale);
  const [contrast, setContrast] = useState(highContrast);
  const [motion, setMotion] = useState(reducedMotion);

  return (
    <SettingsGroup title={t("accessibility.title")}>
      <SettingsRow stacked title={t("accessibility.scale")}>
        <div className={styles.scale}>
          <Slider
            ariaLabel={t("accessibility.scale")}
            value={scale}
            min={SCALE_MIN}
            max={SCALE_MAX}
            step={0.05}
            wide
            format={(value) => `${Math.round(value * 100)} %`}
            onChange={(value) => {
              setScale(value);
              setUiScale(value);
            }}
          />
          <div className={styles.ends}>
            <span>{t("accessibility.smaller")}</span>
            <span>{t("accessibility.larger")}</span>
          </div>
        </div>
      </SettingsRow>
      <SettingsRow title={t("accessibility.contrast")} description={t("accessibility.contrastHint")}>
        <Switch
          ariaLabel={t("accessibility.contrast")}
          checked={contrast}
          onChange={(on) => {
            setContrast(on);
            setHighContrast(on);
          }}
        />
      </SettingsRow>
      <SettingsRow title={t("accessibility.motion")} description={t("accessibility.motionHint")}>
        <Switch
          ariaLabel={t("accessibility.motion")}
          checked={motion}
          onChange={(on) => {
            setMotion(on);
            setReducedMotion(on);
          }}
        />
      </SettingsRow>
    </SettingsGroup>
  );
}
