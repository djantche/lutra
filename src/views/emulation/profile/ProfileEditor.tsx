import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Select } from "../../../components/Select/Select";
import { SegmentedControl } from "../../../components/SegmentedControl/SegmentedControl";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../../components/Spinner/Spinner";
import { Switch } from "../../../components/Switch/Switch";
import { useCoreOptions } from "../../../hooks/useCoreOptions";
import { type CoreOption, type EmulationProfile } from "../../../lib/backend";
import { buttonLabel } from "../../../lib/buttonLabels";
import { consoleButtons, DEFAULT_BUTTON } from "../../../lib/consoleButtons";
import { controllerStatus, type ControllerButton, type ControllerFamily } from "../../../lib/controllers";
import { CoreOptionRows } from "./CoreOptionRows";
import styles from "./ProfileEditor.module.scss";

export type ProfileSection = "video" | "controls" | "emulator";

// Which of the emulator's own option categories belong with Lutra's picture and controls.
const VIDEO = /video|graphic|display|screen|render|colou?r|palette|shader/i;
const INPUT = /input|control|pad|joy|button/i;

const isVideo = (option: CoreOption) => VIDEO.test(option.category ?? "");
const isInput = (option: CoreOption) => INPUT.test(option.category ?? "");

/** The physical buttons a console button can be put on. */
const PHYSICAL: ControllerButton[] = [
  "south", "east", "west", "north", "l1", "r1", "l2", "r2", "l3", "r3", "select", "start",
  "dpadUp", "dpadDown", "dpadLeft", "dpadRight",
];

interface ProfileEditorProps {
  system: string;
  section: ProfileSection;
  profile: EmulationProfile;
  onChange: (profile: EmulationProfile) => void;
}

/**
 * A console's (or a game's) settings: Lutra's own picture and controls, and the options the
 * emulator offers, each next to what it is about.
 */
export function ProfileEditor({ system, section, profile, onChange }: ProfileEditorProps) {
  const { t } = useTranslation();
  const { options, error } = useCoreOptions(system);
  const [family, setFamily] = useState<ControllerFamily>("generic");

  useEffect(() => {
    controllerStatus()
      .then((status) => setFamily(status.controllers[0]?.family ?? "generic"))
      .catch(() => {});
  }, []);

  const video = (patch: Partial<EmulationProfile["video"]>) =>
    onChange({ ...profile, video: { ...profile.video, ...patch } });

  const emulatorOptions = () => {
    if (error) return <SettingsGroup title={t("emulation.profile.emulator")}><SettingsRow title={t("emulation.profile.noOptions")} description={error} /></SettingsGroup>;
    if (!options) return <div className={styles.loading}><Spinner /></div>;
    return null;
  };

  if (section === "video") {
    return (
      <div className={styles.section}>
        <SettingsGroup title={t("emulation.profile.picture")}>
          <SettingsRow title={t("emulation.profile.scaling")} description={t(`emulation.profile.scalingHint.${profile.video.scaling}`)}>
            <SegmentedControl
              ariaLabel={t("emulation.profile.scaling")}
              segments={(["sharp", "smooth", "integer"] as const).map((value) => ({ value, label: t(`emulation.profile.scalingValue.${value}`) }))}
              value={profile.video.scaling}
              onChange={(scaling) => video({ scaling })}
            />
          </SettingsRow>
          <SettingsRow title={t("emulation.profile.aspect")} description={t(`emulation.profile.aspectHint.${profile.video.aspect}`)}>
            <SegmentedControl
              ariaLabel={t("emulation.profile.aspect")}
              segments={(["console", "square", "stretch"] as const).map((value) => ({ value, label: t(`emulation.profile.aspectValue.${value}`) }))}
              value={profile.video.aspect}
              onChange={(aspect) => video({ aspect })}
            />
          </SettingsRow>
          <SettingsRow title={t("emulation.profile.effect")} description={t(`emulation.profile.effectHint.${profile.video.effect}`)}>
            <SegmentedControl
              ariaLabel={t("emulation.profile.effect")}
              segments={(["none", "scanlines", "lcd"] as const).map((value) => ({ value, label: t(`emulation.profile.effectValue.${value}`) }))}
              value={profile.video.effect}
              onChange={(effect) => video({ effect })}
            />
          </SettingsRow>
        </SettingsGroup>
        {emulatorOptions()}
        {options && (
          <CoreOptionRows options={options} include={isVideo} profile={profile} onChange={onChange} fallbackTitle={t("emulation.profile.emulatorVideo")} />
        )}
      </div>
    );
  }

  if (section === "controls") {
    const remap = (button: keyof typeof DEFAULT_BUTTON, physical: ControllerButton) => {
      const next = { ...profile.controls.remap };
      if (physical === DEFAULT_BUTTON[button]) delete next[button];
      else next[button] = physical;
      onChange({ ...profile, controls: { ...profile.controls, remap: next } });
    };
    const remapped = Object.keys(profile.controls.remap).length > 0;
    return (
      <div className={styles.section}>
        <SettingsGroup title={t("emulation.profile.buttons")}>
          {consoleButtons(system).map(([button, label]) => (
            <SettingsRow key={button} title={label} description={t("emulation.profile.buttonHint")}>
              <Select
                ariaLabel={label}
                value={profile.controls.remap[button] ?? DEFAULT_BUTTON[button]}
                options={PHYSICAL.map((physical) => ({ value: physical, label: buttonLabel(physical, family, t) }))}
                onChange={(physical) => remap(button, physical)}
              />
            </SettingsRow>
          ))}
          <SettingsRow title={t("emulation.profile.stickAsDpad")} description={t("emulation.profile.stickAsDpadHint")}>
            <Switch
              ariaLabel={t("emulation.profile.stickAsDpad")}
              checked={profile.controls.stickAsDpad}
              onChange={(stickAsDpad) => onChange({ ...profile, controls: { ...profile.controls, stickAsDpad } })}
            />
          </SettingsRow>
          {remapped && (
            <SettingsRow title={t("emulation.profile.resetButtons")}>
              <Button variant="ghost" icon="refresh" onClick={() => onChange({ ...profile, controls: { ...profile.controls, remap: {} } })}>
                {t("emulation.profile.reset")}
              </Button>
            </SettingsRow>
          )}
        </SettingsGroup>
        {emulatorOptions()}
        {options && (
          <CoreOptionRows options={options} include={isInput} profile={profile} onChange={onChange} fallbackTitle={t("emulation.profile.emulatorInput")} />
        )}
      </div>
    );
  }

  const changed = Object.keys(profile.coreOptions).length > 0;
  return (
    <div className={styles.section}>
      {emulatorOptions()}
      {options && (
        <>
          <CoreOptionRows
            options={options}
            include={(option) => !isVideo(option) && !isInput(option)}
            profile={profile}
            onChange={onChange}
            fallbackTitle={t("emulation.profile.emulator")}
          />
          {options.options.length === 0 && (
            <SettingsGroup title={t("emulation.profile.emulator")}>
              <SettingsRow title={t("emulation.profile.noOptions")} description={t("emulation.profile.noOptionsHint")} />
            </SettingsGroup>
          )}
          {changed && (
            <SettingsGroup title={t("emulation.profile.defaults")}>
              <SettingsRow title={t("emulation.profile.resetOptions")} description={t("emulation.profile.resetOptionsHint")}>
                <Button variant="ghost" icon="refresh" onClick={() => onChange({ ...profile, coreOptions: {} })}>
                  {t("emulation.profile.reset")}
                </Button>
              </SettingsRow>
            </SettingsGroup>
          )}
        </>
      )}
    </div>
  );
}
