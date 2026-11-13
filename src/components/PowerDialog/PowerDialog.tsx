import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { currentWindow, isHosted } from "../../lib/host";
import { powerOffController, powerTarget, type PowerTarget } from "../../lib/controllers";
import { useToastStore, reportError } from "../../state/toasts";
import { useUi } from "../../state/ui";
import { Button } from "../Button/Button";
import { Dialog } from "../Dialog/Dialog";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./PowerDialog.module.scss";

interface Choice {
  id: string;
  icon: IconName;
  title: string;
  hint: string;
  disabled?: boolean;
  /** The one the selection starts on: never the destructive choice. */
  first?: boolean;
  run: () => void;
}

/**
 * The power menu, opened by holding the controller's Guide button for a few seconds (like a
 * console's): quit Lutra, go back to the desktop mode, minimize, or switch the controller off.
 */
export function PowerDialog() {
  const { t } = useTranslation();
  const { mode, setMode, setPowerMenuOpen } = useUi();
  const [target, setTarget] = useState<PowerTarget | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);

  // The controller whose Guide button was held (what the last choice acts on).
  useEffect(() => {
    powerTarget()
      .then(setTarget)
      .catch(() => setTarget(null));
  }, []);

  const close = () => setPowerMenuOpen(false);
  const leave = (action: () => Promise<void>) => () => {
    close();
    if (isHosted()) action().catch((error) => reportError(t("power.error"), error));
  };

  const switchOff = () => {
    if (!target || busy) return;
    setBusy(true);
    powerOffController(target.id)
      .then(() => {
        useToastStore.getState().notify("info", t("power.switchedOff", { name: target.name }));
        close();
      })
      .catch((error) => {
        setBusy(false);
        const reason = error instanceof Error ? error.message : String(error);
        reportError(t("power.offFailed", { name: target.name }), reason === "wired" ? t("power.wiredHint") : reason);
      });
  };

  const choices: Choice[] = [
    {
      id: "quit",
      icon: "power",
      title: t("power.quit"),
      hint: t("power.quitHint"),
      run: leave(() => currentWindow().quit()),
    },
    ...(mode === "bigpicture"
      ? [
          {
            id: "desktop",
            icon: "monitor" as const,
            title: t("power.desktop"),
            hint: t("power.desktopHint"),
            first: true,
            run: () => {
              setMode("classic");
              close();
            },
          },
        ]
      : []),
    {
      id: "minimize",
      icon: "minimize",
      title: t("power.minimize"),
      hint: t("power.minimizeHint"),
      first: mode !== "bigpicture",
      run: leave(() => currentWindow().hideToTray()),
    },
    ...(target
      ? [
          {
            id: "pad",
            icon: "gamepad" as const,
            title: t("power.pad", { name: target.name }),
            hint: target.wired ? t("power.wiredHint") : t("power.padHint"),
            disabled: target.wired || busy,
            run: switchOff,
          },
        ]
      : []),
  ];

  return (
    <Dialog
      title={t("power.title")}
      description={t("power.description")}
      onClose={close}
      busy={busy}
      footer={
        <Button variant="ghost" onClick={close}>
          {t("common.cancel")}
        </Button>
      }
    >
      <ul className={styles.list}>
        {choices.map((choice) => (
          <li key={choice.id}>
            <button
              type="button"
              data-focusable
              data-autofocus={choice.first ? "" : undefined}
              className={styles.choice}
              disabled={choice.disabled}
              onClick={choice.run}
            >
              <span className={styles.icon}>
                <Icon name={choice.icon} size={20} />
              </span>
              <span className={styles.text}>
                <strong>{choice.title}</strong>
                <small>{choice.hint}</small>
              </span>
            </button>
          </li>
        ))}
      </ul>
    </Dialog>
  );
}
