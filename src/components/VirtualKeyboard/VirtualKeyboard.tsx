import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../Icon/Icon";
import { hoverFocus } from "../../input/hoverFocus";
import { cx } from "../../lib/cx";
import styles from "./VirtualKeyboard.module.scss";

export type KeyboardLayout = "qwerty" | "azerty";

/** A key: what it types (and with Shift), or a function key. */
type Key =
  | { type: "char"; value: string; shifted?: string; span?: number }
  | { type: "backspace" | "clear" | "shift" | "space" | "submit" | "hide"; span: number };

const char = (value: string, shifted?: string): Key => ({ type: "char", value, shifted });
const letters = (text: string): Key[] => text.split("").map((letter) => char(letter));

const DIGITS: Key[] = [
  char("1", "!"),
  char("2", "@"),
  char("3", "#"),
  char("4", "$"),
  char("5", "%"),
  char("6", "^"),
  char("7", "&"),
  char("8", "*"),
  char("9", "("),
  char("0", ")"),
  char("-", "_"),
  char("=", "+"),
  { type: "backspace", span: 2 },
];

// Every row is 14 units wide on the grid (a letter is one).
const ROWS: Record<KeyboardLayout, Key[][]> = {
  qwerty: [
    DIGITS,
    [...letters("qwertyuiop"), char("[", "{"), char("]", "}"), { type: "clear", span: 2 }],
    [...letters("asdfghjkl"), char(";", ":"), char("'", '"'), { type: "submit", span: 3 }],
    [{ type: "shift", span: 3 }, ...letters("zxcvbnm"), char(",", "<"), char(".", ">"), char("/", "?"), char("@", "~")],
    [{ type: "hide", span: 2 }, { type: "space", span: 10 }, char(".", ">"), char("_", "\\")],
  ],
  azerty: [
    DIGITS,
    [...letters("azertyuiop"), char("[", "{"), char("]", "}"), { type: "clear", span: 2 }],
    [...letters("qsdfghjklm"), char("'", '"'), char(";", ":"), { type: "submit", span: 2 }],
    [{ type: "shift", span: 3 }, ...letters("wxcvbn"), char(",", "?"), char(";", "."), char(":", "/"), char("!", "§"), char("@", "~")],
    [{ type: "hide", span: 2 }, { type: "space", span: 10 }, char(".", ">"), char("_", "\\")],
  ],
};

interface VirtualKeyboardProps {
  layout: KeyboardLayout;
  onInsert: (text: string) => void;
  onBackspace: () => void;
  onClear: () => void;
  /** The "search" key: done typing. */
  onSubmit: () => void;
  onHide: () => void;
}

/**
 * An on-screen keyboard for the controller: every key is a stop of the usual navigation, and
 * confirming one types it. Shift applies to the next character only.
 */
export function VirtualKeyboard({ layout, onInsert, onBackspace, onClear, onSubmit, onHide }: VirtualKeyboardProps) {
  const { t } = useTranslation();
  const [shift, setShift] = useState(false);
  const home = useRef<HTMLButtonElement>(null);

  // The selection starts in the middle of the letters, where the hands rest.
  useEffect(() => home.current?.focus({ preventScroll: true }), []);

  const press = (key: Key) => {
    switch (key.type) {
      case "char":
        onInsert(shift ? (key.shifted ?? key.value.toUpperCase()) : key.value);
        setShift(false);
        return;
      case "backspace":
        return onBackspace();
      case "clear":
        return onClear();
      case "shift":
        return setShift((on) => !on);
      case "space":
        return onInsert(" ");
      case "submit":
        return onSubmit();
      case "hide":
        return onHide();
    }
  };

  const label = (key: Key) => {
    switch (key.type) {
      case "char":
        return shift ? (key.shifted ?? key.value.toUpperCase()) : key.value;
      case "backspace":
        return t("keyboard.backspace");
      case "clear":
        return t("keyboard.clear");
      case "shift":
        return t("keyboard.shift");
      case "space":
        return t("keyboard.space");
      case "submit":
        return t("keyboard.search");
      case "hide":
        return <Icon name="chevronDown" size={26} />;
    }
  };

  return (
    <div className={styles.dock}>
      <div className={styles.keyboard} role="group" aria-label={t("keyboard.label")}>
        {ROWS[layout].flat().map((key, index) => (
          <button
            key={index}
            ref={key.type === "char" && key.value === "g" ? home : undefined}
            type="button"
            data-focusable
            className={cx(
              styles.key,
              key.type !== "char" && styles.function,
              key.type === "shift" && shift && styles.on,
              key.type === "submit" && styles.submit,
            )}
            style={{ gridColumn: `span ${key.type === "char" ? 1 : key.span}` }}
            aria-label={key.type === "hide" ? t("keyboard.hide") : undefined}
            aria-pressed={key.type === "shift" ? shift : undefined}
            onClick={() => press(key)}
            {...hoverFocus}
          >
            {label(key)}
          </button>
        ))}
      </div>
    </div>
  );
}
