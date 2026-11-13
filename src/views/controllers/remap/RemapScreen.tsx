import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { ButtonGlyph } from "../../../components/ButtonGlyph/ButtonGlyph";
import { GamepadVisual } from "../../../components/GamepadVisual/GamepadVisual";
import { Icon } from "../../../components/Icon/Icon";
import { Select } from "../../../components/Select/Select";
import { useInputAction } from "../../../input/useInputAction";
import { buttonLabel, keyLabel } from "../../../lib/buttonLabels";
import {
  assignButton,
  controllerPreview,
  type Assignment,
  type ControllerButton,
  type ControllerInfo,
  type ControllerProfile,
  type PadPreview,
} from "../../../lib/controllers";
import { playSound } from "../../../lib/sounds";
import { ButtonCallouts } from "../test/ButtonCallouts";
import testStyles from "../test/ControllerTestScreen.module.scss";
import { useControllerKeys } from "../useControllerKeys";
import styles from "./RemapScreen.module.scss";

const REFRESH_MS = 33;

const LEFT: ControllerButton[] = ["l2", "l1", "dpadUp", "dpadLeft", "dpadRight", "dpadDown", "l3", "select"];
const RIGHT: ControllerButton[] = ["r2", "r1", "north", "west", "east", "south", "r3", "start"];
/** The order "Remap all" goes through: face buttons first, as people expect. */
const ORDER: ControllerButton[] = [
  "south", "east", "west", "north", "l1", "r1", "l2", "r2", "select", "start", "l3", "r3",
  "dpadUp", "dpadDown", "dpadLeft", "dpadRight",
];

/** What is being remapped: nothing, one button, or all of them in turn. */
type Mode = { kind: "idle" } | { kind: "one"; button: ControllerButton } | { kind: "all"; index: number };

interface RemapScreenProps {
  /** The controller to listen to; without one, buttons are assigned from the list only. */
  controller: ControllerInfo | null;
  profile: ControllerProfile;
  onChange: (patch: Pick<ControllerProfile, "remap" | "keys">) => void;
  onClose: () => void;
}

/**
 * A page of its own for the buttons: the controller in the middle, what each button does on the
 * sides. Click a button, then press the one it should act as on the controller; or "Remap all" to go
 * through every button in turn. Escape (or Cancel) stops at any moment.
 */
export function RemapScreen({ controller, profile, onChange, onClose }: RemapScreenProps) {
  const { t } = useTranslation();
  const keys = useControllerKeys();
  const [preview, setPreview] = useState<PadPreview | null>(null);
  const [mode, setMode] = useState<Mode>({ kind: "idle" });
  const family = controller?.family ?? "generic";
  // With a keyboard-only output there is no gamepad to send a button to.
  const gamepad = profile.output !== "keyboard" && profile.output !== "none";

  const target: ControllerButton | null =
    mode.kind === "one" ? mode.button : mode.kind === "all" ? ORDER[mode.index] : null;
  const listening = target !== null;

  // The profile is read through a ref by the polling below, which must not restart on every edit.
  const profileRef = useRef(profile);
  profileRef.current = profile;

  useEffect(() => {
    if (!controller) return;
    let current = true;
    const timer = window.setInterval(() => {
      controllerPreview(controller.id, profileRef.current)
        .then((next) => current && setPreview(next))
        .catch(() => {});
    }, REFRESH_MS);
    return () => {
      current = false;
      window.clearInterval(timer);
    };
  }, [controller]);

  const pressed = preview?.raw.pressed ?? [];

  const assign = useCallback(
    (button: ControllerButton, to: Assignment) => {
      onChange(assignButton(profileRef.current, button, to));
    },
    [onChange],
  );

  /** Moves on after a button was set (or skipped): the next one, or the end. */
  const advance = useCallback(() => {
    setMode((current) => {
      if (current.kind === "all" && current.index + 1 < ORDER.length) return { kind: "all", index: current.index + 1 };
      return { kind: "idle" };
    });
  }, []);

  // A button that goes down while listening is the answer. Buttons already held when listening
  // starts do not count, so the click that began it cannot answer itself.
  const held = useRef<Set<ControllerButton>>(new Set());
  useEffect(() => {
    const now = new Set(pressed);
    const fresh = [...now].filter((button) => !held.current.has(button) && button !== "guide");
    held.current = now;
    if (!target || !gamepad || fresh.length === 0) return;
    assign(target, { kind: "button", button: fresh[0] });
    playSound("confirm");
    advance();
  }, [pressed, target, gamepad, assign, advance]);

  const cancel = () => setMode({ kind: "idle" });

  // While listening the controller answers, so it must not also drive the interface. Otherwise
  // the back button leaves, and everything else navigates as usual.
  useInputAction((action) => {
    if (listening) return true;
    if (action === "back") {
      onClose();
      return true;
    }
  });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      if (listening) {
        playSound("back");
        cancel();
      } else onClose();
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [listening, onClose]);

  const host = document.querySelector("[data-overlay-root]") ?? document.body;
  const changedCount = [...LEFT, ...RIGHT].filter(
    (button) => profile.remap[button] !== undefined || profile.keys[button] !== undefined,
  ).length;

  return createPortal(
    <div className={testStyles.screen} role="dialog" aria-modal="true" aria-label={t("controllers.remapScreen.title")} data-nav-trap>
      <header className={testStyles.header}>
        <div>
          <h2>{t("controllers.remapScreen.title")}</h2>
          <p>{t(`controllers.output.${profile.output}`)}</p>
        </div>
        <div className={styles.actions}>
          {gamepad && controller && (
            <Button variant="primary" icon="gamepad" disabled={listening} onClick={() => setMode({ kind: "all", index: 0 })}>
              {t("controllers.remapScreen.all")}
            </Button>
          )}
          <Button variant="ghost" disabled={changedCount === 0 || listening} onClick={() => onChange({ remap: {}, keys: {} })}>
            {t("controllers.remapScreen.reset")}
          </Button>
          <button type="button" data-focusable className={testStyles.close} aria-label={t("controllers.test.close")} onClick={onClose}>
            <Icon name="close" size={20} />
          </button>
        </div>
      </header>

      <div className={testStyles.stage}>
        <ButtonCallouts
          side="left"
          buttons={LEFT}
          profile={profile}
          family={family}
          pressed={pressed}
          active={target}
          onPick={(button) => setMode({ kind: "one", button })}
        />
        <div className={testStyles.pad}>
          <GamepadVisual state={preview?.raw ?? null} family={family} model={controller?.model} />
        </div>
        <ButtonCallouts
          side="right"
          buttons={RIGHT}
          profile={profile}
          family={family}
          pressed={pressed}
          active={target}
          onPick={(button) => setMode({ kind: "one", button })}
        />
      </div>

      <section className={styles.panel} aria-live="polite">
        {target ? (
          <>
            <div className={styles.prompt}>
              <ButtonGlyph button={target} family={family} />
              <div>
                <strong>
                  {mode.kind === "all"
                    ? t("controllers.remapScreen.step", { current: mode.index + 1, total: ORDER.length })
                    : t("controllers.remapScreen.one")}
                </strong>
                <p>
                  {gamepad && controller
                    ? t("controllers.remapScreen.press", { button: buttonLabel(target, family, t) })
                    : t("controllers.remapScreen.choose")}
                </p>
              </div>
            </div>
            <div className={styles.choices}>
              <div className={styles.keyPick}>
                <Select
                  ariaLabel={t("controllers.remapScreen.key")}
                  value=""
                  placeholder={t("controllers.remapScreen.key")}
                  onChange={(key) => {
                    assign(target, { kind: "key", key });
                    advance();
                  }}
                  options={keys.map((info) => ({
                    value: info.name,
                    label: keyLabel(info.name, t),
                    hint: t(`controllers.keyGroups.${info.group}`),
                  }))}
                />
              </div>
              <Button
                variant="ghost"
                onClick={() => {
                  assign(target, { kind: "off" });
                  advance();
                }}
              >
                {t("controllers.remapScreen.disable")}
              </Button>
              {mode.kind === "all" && (
                <Button variant="ghost" onClick={advance}>
                  {t("controllers.remapScreen.skip")}
                </Button>
              )}
              <Button variant="secondary" onClick={cancel}>
                {t("controllers.remapScreen.cancel")}
              </Button>
            </div>
          </>
        ) : (
          <p className={styles.hint}>{controller ? t("controllers.remapScreen.hint") : t("controllers.remapScreen.noController")}</p>
        )}
      </section>
    </div>,
    host,
  );
}
