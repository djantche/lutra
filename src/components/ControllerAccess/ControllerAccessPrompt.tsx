import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { hasBackend } from "../../lib/backend";
import { controllerStatus, onControllersChanged } from "../../lib/controllers";
import { Button } from "../Button/Button";
import { Dialog } from "../Dialog/Dialog";
import { ControllerAccessButton } from "./ControllerAccessButton";

const LATER = "lutra.controllerAccess.later";

const answered = (): boolean => {
  try {
    return localStorage.getItem(LATER) === "1";
  } catch {
    return false;
  }
};

/**
 * The first time a controller is connected and the system does not let Lutra make virtual
 * controllers, offers to set it up (one click, the system asks for the password). "Not now" is
 * remembered; the Controllers page still offers it.
 */
export function ControllerAccessPrompt() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!hasBackend() || answered()) return;
    const check = () =>
      controllerStatus()
        .then((status) => {
          if (status.controllers.length > 0 && (!status.uinput || !status.uhid)) setOpen(true);
        })
        .catch(() => {});
    check();
    const stop = onControllersChanged(check);
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  if (!open) return null;

  const later = () => {
    try {
      localStorage.setItem(LATER, "1");
    } catch {
      // Asked again next time.
    }
    setOpen(false);
  };

  return (
    <Dialog
      title={t("controllers.access.title")}
      description={t("controllers.access.hint")}
      onClose={later}
      footer={
        <Button variant="ghost" onClick={later}>
          {t("controllers.access.later")}
        </Button>
      }
    >
      <ControllerAccessButton autoFocus onDone={() => setOpen(false)} />
    </Dialog>
  );
}
