import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup, SettingsRow } from "../../components/SettingsGroup/SettingsGroup";
import { Switch } from "../../components/Switch/Switch";
import { setControllerNavigation, setControllerSystem } from "../../lib/controllers";
import { reportError } from "../../state/toasts";
import { ControllerList } from "./ControllerList";
import { ControllerNotice } from "./ControllerNotice";
import { LiveTester } from "./LiveTester";
import { ProfileEditor } from "./ProfileEditor";
import { useControllerProfile, useControllerStatus } from "./useControllers";
import styles from "./Controllers.module.scss";

/** The controller driver's home: connected controllers, live test, and the default profile. */
export function ControllersView() {
  const { t } = useTranslation();
  const { status, refresh } = useControllerStatus();
  const { profile, update } = useControllerProfile(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const controllers = status?.controllers ?? [];
  const selected = controllers.find((controller) => controller.id === selectedId) ?? controllers[0] ?? null;

  useEffect(() => {
    if (selected && selected.id !== selectedId) setSelectedId(selected.id);
  }, [selected, selectedId]);

  return (
    <div className={styles.view}>
      <header className={styles.header}>
        <h1>{t("controllers.title")}</h1>
        <p>{t("controllers.subtitle")}</p>
      </header>

      {status && <ControllerNotice status={status} />}

      {status && selected && profile && (
        <div className={styles.page}>
          {controllers.length > 1 && (
            <ControllerList controllers={controllers} selectedId={selected.id} onSelect={setSelectedId} />
          )}
          <LiveTester controller={selected} status={status} profile={profile} />
          <div className={styles.groups}>
            <SettingsGroup title={t("controllers.everywhere")}>
              <SettingsRow title={t("controllers.system")} description={t("controllers.systemHint")}>
                <Switch
                  ariaLabel={t("controllers.system")}
                  checked={status.system}
                  disabled={!status.uinput}
                  onChange={(enabled) =>
                    setControllerSystem(enabled)
                      .then(refresh)
                      .catch((error) => reportError(t("errors.save"), error))
                  }
                />
              </SettingsRow>
            </SettingsGroup>
            <SettingsGroup title={t("controllers.menus")}>
              <SettingsRow title={t("controllers.navigation")} description={t("controllers.navigationHint")}>
                <Switch
                  ariaLabel={t("controllers.navigation")}
                  checked={status.navigation}
                  onChange={(enabled) =>
                    setControllerNavigation(enabled)
                      .then(refresh)
                      .catch((error) => reportError(t("errors.save"), error))
                  }
                />
              </SettingsRow>
            </SettingsGroup>
          </div>

          <p className={styles.scope}>{t("controllers.defaultScope")}</p>
          <ProfileEditor profile={profile} controller={selected} onChange={update} />
        </div>
      )}
    </div>
  );
}
