import { useTranslation } from "react-i18next";
import { Icon } from "../../components/Icon/Icon";
import type { ControllerStatus } from "../../lib/controllers";
import styles from "./Controllers.module.scss";

const UDEV_RULE =
  'echo \'KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"\' | sudo tee /etc/udev/rules.d/60-lutra-uinput.rules';

/** `/dev/uhid` for Lutra's HID controllers, and their `hidraw` nodes for the games. */
const UHID_RULE =
  "printf '%s\\n' 'KERNEL==\"uhid\", SUBSYSTEM==\"misc\", TAG+=\"uaccess\", OPTIONS+=\"static_node=uhid\"' " +
  "'KERNEL==\"hidraw*\", KERNELS==\"*054C:05C4*\", MODE=\"0660\", TAG+=\"uaccess\"' " +
  "'KERNEL==\"hidraw*\", KERNELS==\"*054C:09CC*\", MODE=\"0660\", TAG+=\"uaccess\"' " +
  "| sudo tee /etc/udev/rules.d/60-lutra-uhid.rules && sudo udevadm control --reload && sudo udevadm trigger --name-match=uhid";

/** What keeps the driver from working, if anything, and how to fix it. */
export function ControllerNotice({ status }: { status: ControllerStatus }) {
  const { t } = useTranslation();

  if (!status.uinput) {
    return (
      <div className={styles.notice} data-kind="warning">
        <Icon name="info" size={20} />
        <div>
          <strong>{t("controllers.noUinput")}</strong>
          <p>{t("controllers.noUinputHint")}</p>
          <code>{UDEV_RULE}</code>
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
          <code>{UHID_RULE}</code>
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
