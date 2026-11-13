import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Select } from "../../../components/Select/Select";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import {
  hardwareInfo,
  recommendedConfig,
  type Gamescope,
  type Hardware,
  type RunnerKind,
  type WineConfig,
} from "../../../lib/backend";
import { reportError } from "../../../state/toasts";
import styles from "./Tabs.module.scss";

const FPS_LIMITS = [30, 45, 60, 75, 90, 120, 144, 165, 240];
const GAMESCOPE_SIZES = ["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];
const DEFAULT_GAMESCOPE: Gamescope = { width: 1920, height: 1080, fullscreen: true, fsr: false };
const SYSTEM_GPU = "";

interface PerformanceTabProps {
  config: WineConfig;
  runnerKind: RunnerKind;
  onConfig: (patch: Partial<WineConfig>) => void;
}

/**
 * What makes a game run well on this machine. Every choice comes from a list built from the
 * hardware that was detected, so the user cannot pick a graphics card or a feature that is not there.
 */
export function PerformanceTab({ config, runnerKind, onConfig }: PerformanceTabProps) {
  const { t } = useTranslation();
  const [hardware, setHardware] = useState<Hardware | null>(null);
  const isProton = runnerKind === "proton";
  const isNative = runnerKind === "native";

  useEffect(() => {
    hardwareInfo()
      .then(setHardware)
      .catch(() => setHardware(null));
  }, []);

  const gpus = hardware?.gpus ?? [];
  const hasNvidia = gpus.some((gpu) => gpu.vendor === "nvidia");
  const gamescopeInstalled = Boolean(hardware?.gamescope);

  const applyRecommended = () =>
    recommendedConfig(runnerKind)
      .then((recommended) =>
        onConfig({
          gpu: recommended.gpu,
          ntsync: recommended.ntsync,
          nvapi: recommended.nvapi,
          perfTweaks: recommended.perfTweaks,
          gamemode: recommended.gamemode,
          esync: recommended.esync,
          fsync: recommended.fsync,
        }),
      )
      .catch((error) => reportError(t("errors.settings"), error));

  const gamescope = config.gamescope;
  const gamescopeSize = gamescope ? `${gamescope.width}x${gamescope.height}` : "1920x1080";

  return (
    <>
      <SettingsGroup title={t("perf.recommended")}>
        <SettingsRow title={t("perf.optimize")} description={t("perf.optimizeHint")}>
          <Button variant="secondary" onClick={applyRecommended}>
            {t("perf.apply")}
          </Button>
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title={t("perf.graphics")}>
        {gpus.length > 0 && (
          <SettingsRow title={t("perf.gpu")} description={gpus.length > 1 ? t("perf.gpuHint") : t("perf.gpuSingle")}>
            <div className={styles.control}>
              <Select
                ariaLabel={t("perf.gpu")}
                value={config.gpu ?? SYSTEM_GPU}
                disabled={gpus.length < 2}
                options={[
                  { value: SYSTEM_GPU, label: t("perf.gpuSystem"), hint: t("perf.gpuSystemHint") },
                  ...gpus.map((gpu) => ({
                    value: gpu.id,
                    label: gpu.name,
                    hint: gpu.primary ? t("perf.gpuPrimary") : t("perf.gpuSecondary"),
                  })),
                ]}
                onChange={(value) => onConfig({ gpu: value === SYSTEM_GPU ? null : value })}
              />
            </div>
          </SettingsRow>
        )}

        <SettingsRow title={t("perf.fpsLimit")} description={t("perf.fpsLimitHint")}>
          <div className={styles.control}>
            <Select
              ariaLabel={t("perf.fpsLimit")}
              value={String(config.fpsLimit ?? 0)}
              options={[
                { value: "0", label: t("perf.off") },
                ...[...new Set([...FPS_LIMITS, ...(config.fpsLimit ? [config.fpsLimit] : [])])]
                  .sort((a, b) => a - b)
                  .map((limit) => ({ value: String(limit), label: `${limit} FPS` })),
              ]}
              onChange={(value) => onConfig({ fpsLimit: Number(value) || null })}
            />
          </div>
        </SettingsRow>

        {!isNative && (
          <SettingsRow title={t("perf.hud")} description={t("perf.hudHint")}>
            <div className={styles.control}>
              <Select
                ariaLabel={t("perf.hud")}
                value={config.dxvkHud ?? ""}
                options={[
                  { value: "", label: t("perf.off") },
                  { value: "fps", label: t("perf.hudFps") },
                  { value: "fps,frametimes,gpuload", label: t("perf.hudDetailed") },
                  { value: "full", label: t("perf.hudFull") },
                ]}
                onChange={(value) => onConfig({ dxvkHud: value || null })}
              />
            </div>
          </SettingsRow>
        )}

        {isProton && hasNvidia && (
          <SettingsRow title={t("perf.nvapi")} description={t("perf.nvapiHint")}>
            <Switch ariaLabel={t("perf.nvapi")} checked={config.nvapi} onChange={(nvapi) => onConfig({ nvapi })} />
          </SettingsRow>
        )}
        {isProton && (
          <>
            <SettingsRow title={t("perf.wayland")} description={t("perf.waylandHint")}>
              <Switch ariaLabel={t("perf.wayland")} checked={config.wayland} onChange={(wayland) => onConfig({ wayland })} />
            </SettingsRow>
            <SettingsRow title={t("perf.fsr")} description={t("perf.fsrHint")}>
              <Switch ariaLabel={t("perf.fsr")} checked={config.fsr} onChange={(fsr) => onConfig({ fsr })} />
            </SettingsRow>
          </>
        )}
      </SettingsGroup>

      <SettingsGroup title={t("perf.system")}>
        {isProton && (
          <SettingsRow
            title={t("perf.ntsync")}
            description={hardware && !hardware.ntsync ? t("perf.ntsyncMissing") : t("perf.ntsyncHint")}
          >
            <Switch
              ariaLabel={t("perf.ntsync")}
              checked={config.ntsync}
              disabled={hardware !== null && !hardware.ntsync}
              onChange={(ntsync) => onConfig({ ntsync })}
            />
          </SettingsRow>
        )}
        <SettingsRow title={t("perf.tweaks")} description={t("perf.tweaksHint")}>
          <Switch ariaLabel={t("perf.tweaks")} checked={config.perfTweaks} onChange={(perfTweaks) => onConfig({ perfTweaks })} />
        </SettingsRow>
      </SettingsGroup>

      <SettingsGroup title="Gamescope">
        <SettingsRow
          title={t("perf.gamescope")}
          description={hardware && !gamescopeInstalled ? t("config.notInstalled") : t("perf.gamescopeHint")}
        >
          <Switch
            ariaLabel={t("perf.gamescope")}
            checked={gamescope !== null}
            disabled={!gamescopeInstalled}
            onChange={(enabled) => onConfig({ gamescope: enabled ? DEFAULT_GAMESCOPE : null })}
          />
        </SettingsRow>
        {gamescope && (
          <>
            <SettingsRow title={t("perf.gamescopeSize")} description={t("perf.gamescopeSizeHint")}>
              <div className={styles.size}>
                <Select
                  ariaLabel={t("perf.gamescopeSize")}
                  value={gamescopeSize}
                  options={(GAMESCOPE_SIZES.includes(gamescopeSize) ? GAMESCOPE_SIZES : [gamescopeSize, ...GAMESCOPE_SIZES]).map(
                    (size) => ({ value: size, label: size.replace("x", " × ") }),
                  )}
                  onChange={(size) => {
                    const [width, height] = size.split("x").map(Number);
                    onConfig({ gamescope: { ...gamescope, width, height } });
                  }}
                />
              </div>
            </SettingsRow>
            <SettingsRow title={t("perf.gamescopeFullscreen")}>
              <Switch
                ariaLabel={t("perf.gamescopeFullscreen")}
                checked={gamescope.fullscreen}
                onChange={(fullscreen) => onConfig({ gamescope: { ...gamescope, fullscreen } })}
              />
            </SettingsRow>
            <SettingsRow title={t("perf.gamescopeFsr")} description={t("perf.gamescopeFsrHint")}>
              <Switch
                ariaLabel={t("perf.gamescopeFsr")}
                checked={gamescope.fsr}
                onChange={(fsr) => onConfig({ gamescope: { ...gamescope, fsr } })}
              />
            </SettingsRow>
          </>
        )}
      </SettingsGroup>
    </>
  );
}
