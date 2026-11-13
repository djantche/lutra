import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import { Select } from "../../components/Select/Select";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Slider } from "../../components/Slider/Slider";
import { Switch } from "../../components/Switch/Switch";
import {
  audioStatus,
  setAudioDevice,
  setAudioMuted,
  setAudioVolume,
  type AudioKind,
  type AudioSide,
  type AudioStatus,
} from "../../lib/audio";
import { setSoundsEnabled, soundsEnabled } from "../../lib/sounds";
import { reportError } from "../../state/toasts";
import styles from "./AudioSection.module.scss";

const REFRESH_MS = 3000;
/** While the slider is dragged, the system hears about it at most this often. */
const SEND_EVERY_MS = 70;

/** Sound: the speakers and the microphone as the system sees them, and the interface sounds. */
export function AudioSection() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<AudioStatus | null | undefined>(undefined);
  const [sounds, setSounds] = useState(soundsEnabled);
  /** A volume being dragged, shown until the system has caught up. */
  const [pending, setPending] = useState<Partial<Record<AudioKind, number>>>({});
  const lastSent = useRef<Record<AudioKind, number>>({ output: 0, input: 0 });
  const dragging = useRef(false);

  const refresh = useCallback(() => {
    if (dragging.current) return;
    void audioStatus().then(setStatus);
  }, []);

  useEffect(() => {
    refresh();
    const timer = window.setInterval(refresh, REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const volume = (kind: AudioKind, percent: number) => {
    dragging.current = true;
    setPending((current) => ({ ...current, [kind]: percent }));
    const now = performance.now();
    const settle = () => {
      dragging.current = false;
      setPending((current) => ({ ...current, [kind]: undefined }));
      refresh();
    };
    if (now - lastSent.current[kind] >= SEND_EVERY_MS) {
      lastSent.current[kind] = now;
      setAudioVolume(kind, percent).then(settle).catch((error) => reportError(t("errors.save"), error));
    } else {
      // Between two sends: the last value is sent when the slider has been quiet a moment.
      window.clearTimeout(timers.current[kind]);
      timers.current[kind] = window.setTimeout(() => {
        lastSent.current[kind] = performance.now();
        setAudioVolume(kind, percent).then(settle).catch((error) => reportError(t("errors.save"), error));
      }, SEND_EVERY_MS);
    }
  };
  const timers = useRef<Record<AudioKind, number>>({ output: 0, input: 0 });

  const mute = (kind: AudioKind, muted: boolean) =>
    setAudioMuted(kind, muted)
      .then(refresh)
      .catch((error) => reportError(t("errors.save"), error));

  const device = (kind: AudioKind, name: string) =>
    setAudioDevice(kind, name)
      .then(refresh)
      .catch((error) => reportError(t("errors.save"), error));

  const side = (kind: AudioKind, current: AudioSide) => {
    const shown = pending[kind] ?? current.volume;
    const output = kind === "output";
    return (
      <>
        <SettingsRow stacked title={t(output ? "audio.output" : "audio.voice")}>
          <div className={styles.volume}>
            <button
              type="button"
              data-focusable
              className={styles.mute}
              aria-pressed={current.muted}
              aria-label={t(current.muted ? "audio.unmute" : "audio.mute")}
              onClick={() => void mute(kind, !current.muted)}
            >
              <Icon name={output ? (current.muted ? "volumeOff" : "volume") : current.muted ? "micOff" : "mic"} size={22} />
            </button>
            <Slider
              ariaLabel={t(output ? "audio.output" : "audio.voice")}
              value={shown}
              min={0}
              max={100}
              step={1}
              wide
              format={(value) => `${value} %`}
              onChange={(value) => volume(kind, value)}
            />
          </div>
        </SettingsRow>
        <SettingsRow title={t(output ? "audio.outputDevice" : "audio.inputDevice")}>
          <div className={styles.device}>
            <Select
              ariaLabel={t(output ? "audio.outputDevice" : "audio.inputDevice")}
              value={current.default}
              onChange={(name) => void device(kind, name)}
              options={current.devices.map((item) => ({ value: item.name, label: item.description }))}
            />
          </div>
        </SettingsRow>
      </>
    );
  };

  return (
    <>
      {status === null && (
        <SettingsGroup title={t("audio.systemTitle")}>
          <SettingsRow title={t("audio.unavailable")} description={t("audio.unavailableHint")} />
        </SettingsGroup>
      )}
      {status && (
        <>
          <SettingsGroup title={t("audio.output")}>{side("output", status.output)}</SettingsGroup>
          <SettingsGroup title={t("audio.voice")}>{side("input", status.input)}</SettingsGroup>
        </>
      )}
      <SettingsGroup title={t("audio.general")}>
        <SettingsRow title={t("audio.uiSounds")} description={t("settings.soundsHint")}>
          <Switch
            ariaLabel={t("audio.uiSounds")}
            checked={sounds}
            onChange={(enabled) => {
              setSounds(enabled);
              setSoundsEnabled(enabled);
            }}
          />
        </SettingsRow>
      </SettingsGroup>
    </>
  );
}
