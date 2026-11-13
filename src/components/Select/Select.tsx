import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { useInputAction } from "../../input/useInputAction";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import styles from "./Select.module.scss";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
  /** Secondary line: what the option is, or why it cannot be chosen. */
  hint?: string;
  /** Shown but not selectable; the hint says why. */
  disabled?: boolean;
}

interface SelectProps<T extends string> {
  value: T;
  options: SelectOption<T>[];
  onChange: (value: T) => void;
  ariaLabel?: string;
  placeholder?: string;
  disabled?: boolean;
  /** Marks the select as the element to focus when its screen opens. */
  autofocus?: boolean;
}

/** A dropdown that replaces the browser's `<select>`: same keyboard behaviour, styled to the app. */
export function Select<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  placeholder,
  disabled = false,
  autofocus = false,
}: SelectProps<T>) {
  const id = useId();
  const root = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);

  const selected = options.findIndex((option) => option.value === value);
  const optionId = (index: number) => `${id}-${index}`;

  const show = () => {
    setActive(selected >= 0 ? selected : nextEnabled(-1, 1));
    setOpen(true);
  };

  /** The nearest selectable option from `from` going `step` (1 or -1), staying put at the ends. */
  const nextEnabled = (from: number, step: 1 | -1) => {
    for (let index = from + step; index >= 0 && index < options.length; index += step) {
      if (!options[index].disabled) return index;
    }
    return from;
  };

  const choose = (index: number) => {
    const option = options[index];
    if (!option || option.disabled) return;
    onChange(option.value);
    setOpen(false);
  };

  useEffect(() => {
    if (!open) return;
    const close = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);

  // Inside a scrolling form the list can open below the visible part: bring all of it into view.
  useEffect(() => {
    if (open) document.getElementById(`${id}-list`)?.scrollIntoView({ block: "nearest" });
  }, [open, id]);

  useEffect(() => {
    if (open) document.getElementById(optionId(active))?.scrollIntoView({ block: "nearest" });
  });

  // With a controller: up/down move in the open list, confirm picks, back closes.
  useInputAction(
    (action) => {
      switch (action) {
        case "up":
          setActive((index) => nextEnabled(index, -1));
          return true;
        case "down":
          setActive((index) => nextEnabled(index, 1));
          return true;
        case "confirm":
          choose(active);
          return true;
        case "back":
          setOpen(false);
          return true;
      }
    },
    open,
  );

  const onKeyDown = (event: KeyboardEvent) => {
    const move = (index: number) => {
      event.preventDefault();
      setActive(index);
    };

    if (!open) {
      if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
        event.preventDefault();
        show();
      }
      return;
    }

    switch (event.key) {
      case "ArrowDown":
        return move(nextEnabled(active, 1));
      case "ArrowUp":
        return move(nextEnabled(active, -1));
      case "Home":
        return move(nextEnabled(-1, 1));
      case "End":
        return move(nextEnabled(options.length, -1));
      case "Enter":
      case " ":
        event.preventDefault();
        return choose(active);
      case "Escape":
        // Only closes the list, not whatever dialog the select sits in.
        event.preventDefault();
        return setOpen(false);
      case "Tab":
        return setOpen(false);
    }
  };

  return (
    <div ref={root} className={styles.root}>
      <button
        type="button"
        role="combobox"
        data-focusable
        data-autofocus={autofocus ? "" : undefined}
        className={cx(styles.trigger, open && styles.open)}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={`${id}-list`}
        aria-activedescendant={open ? optionId(active) : undefined}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : show())}
        onKeyDown={onKeyDown}
      >
        <span className={selected >= 0 ? styles.value : styles.placeholder}>
          {selected >= 0 ? options[selected].label : placeholder}
        </span>
        <Icon name="chevronDown" size={16} />
      </button>

      {open && (
        <ul id={`${id}-list`} role="listbox" className={styles.list}>
          {options.map((option, index) => (
            <li
              key={option.value}
              id={optionId(index)}
              role="option"
              aria-selected={index === selected}
              aria-disabled={option.disabled || undefined}
              className={cx(styles.option, index === active && !option.disabled && styles.active)}
              // Keeps the focus on the trigger so the keyboard keeps working.
              onPointerDown={(event) => event.preventDefault()}
              onPointerMove={() => !option.disabled && setActive(index)}
              onClick={(event) => {
                // Inside a <label>, the click would be forwarded to the trigger and reopen the list.
                event.preventDefault();
                choose(index);
              }}
            >
              <span className={styles.optionText}>
                <span>{option.label}</span>
                {option.hint && <small>{option.hint}</small>}
              </span>
              {index === selected && <Icon name="check" size={15} />}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
