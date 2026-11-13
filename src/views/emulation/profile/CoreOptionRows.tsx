import { useTranslation } from "react-i18next";
import { Select } from "../../../components/Select/Select";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../../components/Switch/Switch";
import type { CoreOption, CoreOptions, EmulationProfile } from "../../../lib/backend";

const ON = ["enabled", "on", "true", "yes"];
const OFF = ["disabled", "off", "false", "no"];

/** Words cores leave in English ("enabled") said in the interface's language. */
function choiceLabel(label: string, t: (key: string) => string): string {
  const word = label.trim().toLowerCase();
  if (ON.includes(word)) return t("emulation.options.on");
  if (OFF.includes(word)) return t("emulation.options.off");
  return label;
}

/** An on/off option: shown as a switch. Returns the "on" and "off" values. */
function toggleValues(option: CoreOption): [string, string] | null {
  if (option.values.length !== 2) return null;
  const word = (index: number) => option.values[index].label.trim().toLowerCase();
  const on = [0, 1].find((index) => ON.includes(word(index)));
  const off = [0, 1].find((index) => OFF.includes(word(index)));
  return on !== undefined && off !== undefined ? [option.values[on].value, option.values[off].value] : null;
}

interface CoreOptionRowsProps {
  options: CoreOptions;
  /** Which options to show. */
  include: (option: CoreOption) => boolean;
  profile: EmulationProfile;
  onChange: (profile: EmulationProfile) => void;
  /** Title of the options without a category. */
  fallbackTitle: string;
}

/** The emulator's options, as it describes them, grouped by its own categories. */
export function CoreOptionRows({ options, include, profile, onChange, fallbackTitle }: CoreOptionRowsProps) {
  const { t } = useTranslation();
  const shown = options.options.filter((option) => option.visible && include(option));
  if (shown.length === 0) return null;

  const set = (option: CoreOption, value: string) => {
    const coreOptions = { ...profile.coreOptions };
    // The core's default is not stored: a newer core may change it.
    if (value === option.default) delete coreOptions[option.key];
    else coreOptions[option.key] = value;
    onChange({ ...profile, coreOptions });
  };

  // In the core's order of categories, then the options without one.
  const groups = [
    ...options.categories.map((category) => ({
      key: category.key,
      title: category.label || fallbackTitle,
      options: shown.filter((option) => option.category === category.key),
    })),
    {
      key: "",
      title: fallbackTitle,
      options: shown.filter(
        (option) => !option.category || !options.categories.some((category) => category.key === option.category),
      ),
    },
  ].filter((group) => group.options.length > 0);

  return (
    <>
      {groups.map((group) => (
        <SettingsGroup key={group.key || "other"} title={group.title}>
          {group.options.map((option) => {
            const value = profile.coreOptions[option.key] ?? option.default;
            const toggle = toggleValues(option);
            return (
              <SettingsRow key={option.key} title={option.label} description={option.info ?? undefined}>
                {toggle ? (
                  <Switch
                    ariaLabel={option.label}
                    checked={value === toggle[0]}
                    onChange={(on) => set(option, on ? toggle[0] : toggle[1])}
                  />
                ) : (
                  <Select
                    ariaLabel={option.label}
                    value={value}
                    options={option.values.map((choice) => ({ value: choice.value, label: choiceLabel(choice.label, t) }))}
                    onChange={(next) => set(option, next)}
                  />
                )}
              </SettingsRow>
            );
          })}
        </SettingsGroup>
      ))}
    </>
  );
}
