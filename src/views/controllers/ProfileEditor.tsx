import { useTranslation } from "react-i18next";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Slider } from "../../components/Slider/Slider";
import { Switch } from "../../components/Switch/Switch";
import { Select } from "../../components/Select/Select";
import type { ControllerInfo, ControllerProfile, OutputKind, StickMode } from "../../lib/controllers";
import { useState } from "react";
import { Button } from "../../components/Button/Button";
import { RemapScreen } from "./remap/RemapScreen";

const percent = (value: number) => `${Math.round(value * 100)} %`;

interface ProfileEditorProps {
  profile: ControllerProfile;
  /** The controller the buttons are remapped on; without one, they are chosen from lists. */
  controller: ControllerInfo | null;
  onChange: (patch: Partial<ControllerProfile>) => void;
}

/** Everything a profile controls: what the game sees, sticks, triggers, vibrations and buttons. */
export function ProfileEditor({ profile, controller, onChange }: ProfileEditorProps) {
  const { t } = useTranslation();
  const [remapping, setRemapping] = useState(false);
  const remapped = Object.keys(profile.remap).length + Object.keys(profile.keys).length;
  const passthrough = profile.output === "none";
  const keyboardOnly = profile.output === "keyboard";
  const usesMouse = profile.leftStick === "mouse" || profile.rightStick === "mouse";
  const stickModes = (label: string) => [
    { value: "stick" as StickMode, label: keyboardOnly ? t("controllers.stickMode.off") : label },
    { value: "mouse" as StickMode, label: t("controllers.stickMode.mouse") },
    { value: "wasd" as StickMode, label: t("controllers.stickMode.wasd") },
    { value: "arrows" as StickMode, label: t("controllers.stickMode.arrows") },
  ];

  return (
    <>
      <SettingsGroup title={t("controllers.outputTitle")}>
        <SettingsRow stacked title={t("controllers.outputLabel")} description={t(`controllers.outputHint.${profile.output}`)}>
          <SegmentedControl<OutputKind>
            ariaLabel={t("controllers.outputLabel")}
            value={profile.output}
            onChange={(output) => onChange({ output })}
            segments={[
              { value: "auto", label: t("controllers.output.auto") },
              { value: "xbox360", label: t("controllers.output.xbox360") },
              { value: "ds4", label: t("controllers.output.ds4") },
              { value: "keyboard", label: t("controllers.output.keyboard") },
              { value: "none", label: t("controllers.output.none") },
            ]}
          />
        </SettingsRow>
      </SettingsGroup>

      {!passthrough && (
        <>
          <SettingsGroup title={t("controllers.sticks")}>
            <SettingsRow title={t("controllers.leftStickMode")} description={t("controllers.stickModeHint")}>
              <Select
                ariaLabel={t("controllers.leftStickMode")}
                value={profile.leftStick}
                onChange={(leftStick) => onChange({ leftStick })}
                options={stickModes(t("controllers.stickMode.stick"))}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.rightStickMode")} description={t("controllers.stickModeHint")}>
              <Select
                ariaLabel={t("controllers.rightStickMode")}
                value={profile.rightStick}
                onChange={(rightStick) => onChange({ rightStick })}
                options={stickModes(t("controllers.stickMode.stick"))}
              />
            </SettingsRow>
            {usesMouse && (
              <SettingsRow title={t("controllers.mouseSpeed")} description={t("controllers.mouseSpeedHint")}>
                <Slider
                  ariaLabel={t("controllers.mouseSpeed")}
                  value={profile.mouseSpeed}
                  min={200}
                  max={2400}
                  step={50}
                  format={(value) => `${Math.round(value)} px/s`}
                  onChange={(mouseSpeed) => onChange({ mouseSpeed })}
                />
              </SettingsRow>
            )}
            <SettingsRow title={t("controllers.leftDeadzone")} description={t("controllers.deadzoneHint")}>
              <Slider
                ariaLabel={t("controllers.leftDeadzone")}
                value={profile.leftDeadzone}
                min={0}
                max={0.4}
                step={0.01}
                format={percent}
                onChange={(leftDeadzone) => onChange({ leftDeadzone })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.rightDeadzone")} description={t("controllers.deadzoneHint")}>
              <Slider
                ariaLabel={t("controllers.rightDeadzone")}
                value={profile.rightDeadzone}
                min={0}
                max={0.4}
                step={0.01}
                format={percent}
                onChange={(rightDeadzone) => onChange({ rightDeadzone })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.rightCurve")} description={t("controllers.rightCurveHint")}>
              <Slider
                ariaLabel={t("controllers.rightCurve")}
                value={profile.rightCurve}
                min={1}
                max={3}
                step={0.1}
                format={(value) => (value === 1 ? t("controllers.linear") : `× ${value.toFixed(1)}`)}
                onChange={(rightCurve) => onChange({ rightCurve })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.invertLeftY")}>
              <Switch
                ariaLabel={t("controllers.invertLeftY")}
                checked={profile.invertLeftY}
                onChange={(invertLeftY) => onChange({ invertLeftY })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.invertRightY")} description={t("controllers.invertRightYHint")}>
              <Switch
                ariaLabel={t("controllers.invertRightY")}
                checked={profile.invertRightY}
                onChange={(invertRightY) => onChange({ invertRightY })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.swapSticks")} description={t("controllers.swapSticksHint")}>
              <Switch
                ariaLabel={t("controllers.swapSticks")}
                checked={profile.swapSticks}
                onChange={(swapSticks) => onChange({ swapSticks })}
              />
            </SettingsRow>
          </SettingsGroup>

          <SettingsGroup title={t("controllers.triggersAndRumble")}>
            <SettingsRow title={t("controllers.triggerDeadzone")} description={t("controllers.triggerDeadzoneHint")}>
              <Slider
                ariaLabel={t("controllers.triggerDeadzone")}
                value={profile.triggerDeadzone}
                min={0}
                max={0.4}
                step={0.01}
                format={percent}
                onChange={(triggerDeadzone) => onChange({ triggerDeadzone })}
              />
            </SettingsRow>
            <SettingsRow title={t("controllers.rumble")} description={t("controllers.rumbleHint")}>
              <Switch ariaLabel={t("controllers.rumble")} checked={profile.rumble} onChange={(rumble) => onChange({ rumble })} />
            </SettingsRow>
          </SettingsGroup>

          <SettingsGroup title={t("controllers.buttonsTitle")}>
            <SettingsRow title={t("controllers.swapFace")} description={t("controllers.swapFaceHint")}>
              <Switch ariaLabel={t("controllers.swapFace")} checked={profile.swapFace} onChange={(swapFace) => onChange({ swapFace })} />
            </SettingsRow>
            <SettingsRow
              title={t("controllers.remap")}
              description={remapped > 0 ? t("controllers.remapCount", { count: remapped }) : t("controllers.remapHint")}
            >
              <Button variant="secondary" icon="gamepad" onClick={() => setRemapping(true)}>
                {t("controllers.remapOpen")}
              </Button>
            </SettingsRow>
          </SettingsGroup>
        </>
      )}

      {remapping && (
        <RemapScreen controller={controller} profile={profile} onChange={onChange} onClose={() => setRemapping(false)} />
      )}
    </>
  );
}
