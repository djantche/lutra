import { useTranslation } from "react-i18next";
import { ControllerAccessButton } from "../../components/ControllerAccess/ControllerAccessButton";
import { Icon } from "../../components/Icon/Icon";
import type { ControllerStatus } from "../../lib/controllers";
import styles from "./Controllers.module.scss";

/** What keeps the driver from working, if anything, and how to fix it. */
export function ControllerNotice({ status, onFixed }: { status: ControllerStatus; onFixed: () => void }) {
  const { t } = useTranslation();

  if (!status.uinput) {
    return (
      <div className={styles.notice} data-kind="warning">
        <Icon name="info" size={20} />
        <div>
          <strong>{t("controllers.noUinput")}</strong>
          <p>{t("controllers.noUinputHint")}</p>
          <ControllerAccessButton onDone={onFixed} />
        </div>
      </div>
    );
  }

  if (!status.uhid) {
    return (
      <div className={styles.notice} data-kind="warning">
        <Icon name="info" size={20} />
        <div>
          <strong>{t("controllers.noUhid")}</strong>
          <p>{t("controllers.noUhidHint")}</p>
          <ControllerAccessButton onDone={onFixed} />
        </div>
      </div>
    );
  }

  if (status.controllers.length === 0) {
    return (
      <div className={styles.notice}>
        <Icon name="gamepad" size={20} />
        <div>
          <strong>{t("controllers.none")}</strong>
          <p>{t("controllers.noneHint")}</p>
        </div>
      </div>
    );
  }
  return null;
}
