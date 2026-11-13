import { useTranslation } from "react-i18next";
import { SettingsGroup, SettingsRow } from "../../../components/SettingsGroup/SettingsGroup";
import { Spinner } from "../../../components/Spinner/Spinner";
import { Switch } from "../../../components/Switch/Switch";
import { ProfileEditor } from "../../../views/controllers/ProfileEditor";
import { useControllerProfile, useControllerStatus, useLivePad } from "../../../views/controllers/useControllers";

/** The controller profile this game plays with: the default one, or its own. */
export function ControllerTab({ gameId }: { gameId: string }) {
  const { t } = useTranslation();
  const { profile, custom, update, setGameCustom } = useControllerProfile(gameId);
  const { status } = useControllerStatus();
  const controller = status?.controllers[0] ?? null;
  const live = useLivePad(controller?.id ?? null);

  if (!profile) return <Spinner />;

  return (
    <>
      <SettingsGroup title={t("controllers.gameTitle")}>
        <SettingsRow title={t("controllers.gameCustom")} description={t("controllers.gameCustomHint")}>
          <Switch ariaLabel={t("controllers.gameCustom")} checked={custom} onChange={setGameCustom} />
        </SettingsRow>
      </SettingsGroup>

      {custom && (
        <ProfileEditor
          profile={profile}
          family={controller?.family ?? "generic"}
          pressed={live?.pressed ?? []}
          onChange={update}
        />
      )}
    </>
  );
}
