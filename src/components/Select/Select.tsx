import { useEffect, useId, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type WheelEvent } from "react";
import { createPortal } from "react-dom";
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
  const [place, setPlace] = useState<CSSProperties>({});
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLUListElement>(null);
  // The pointer hovering an option makes it active without moving the list under the wheel.
  const pointerActive = useRef(false);

  const selected = options.findIndex((option) => option.value === value);
  const optionId = (index: number) => `${id}-${index}`;

  const show = () => {
    pointerActive.current = false;
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

  // The list is drawn in a layer of its own, above everything, so no parent can cut it. It is
  // placed from the trigger's position: below it, or above when there is more room there, and it
  // follows the trigger while the page scrolls or resizes.
  useLayoutEffect(() => {
    if (!open) return;
    const margin = 8;
    const update = () => {
      const box = trigger.current?.getBoundingClientRect();
      if (!box) return;
      const below = window.innerHeight - box.bottom - margin;
      const above = box.top - margin;
      const goesAbove = below < 200 && above > below;
      const width = Math.min(Math.max(box.width, 180), window.innerWidth - margin * 2);
      const left = Math.min(Math.max(box.left, margin), window.innerWidth - width - margin);
      setPlace({
        left,
        width,
        // The layer is outside the page's own sizes: it takes the trigger's.
        fontSize: trigger.current ? getComputedStyle(trigger.current).fontSize : undefined,
        maxHeight: Math.max(120, Math.min(360, (goesAbove ? above : below) - 6)),
        ...(goesAbove
          ? { bottom: window.innerHeight - box.top + 6, transformOrigin: "bottom" }
          : { top: box.bottom + 6, transformOrigin: "top" }),
      });
    };
    update();
    window.addEventListener("resize", update);
    // The list's own scrolling must not re-place it (that would bring the active option back).
    const onScroll = (event: Event) => {
      if (!(event.target instanceof Node && list.current?.contains(event.target))) update();
    };
    window.addEventListener("scroll", onScroll, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", onScroll, true);
    };
  }, [open]);

  // Keeps the active option in view when the keyboard or the pad moves it, by scrolling the list
  // only (`scrollIntoView` would scroll the page behind as well).
  useEffect(() => {
    if (!open || pointerActive.current) return;
    const box = list.current;
    const option = document.getElementById(optionId(active));
    if (!box || !option) return;
    if (option.offsetTop < box.scrollTop) box.scrollTop = option.offsetTop - 4;
    else if (option.offsetTop + option.offsetHeight > box.scrollTop + box.clientHeight)
      box.scrollTop = option.offsetTop + option.offsetHeight - box.clientHeight + 4;
  }, [open, active]);

  // The wheel over the backdrop scrolls the page under it, which the backdrop would otherwise stop.
  const forwardWheel = (event: WheelEvent<HTMLDivElement>) => {
    for (const element of document.elementsFromPoint(event.clientX, event.clientY)) {
      if (element === event.currentTarget || !(element instanceof HTMLElement)) continue;
      for (let node: HTMLElement | null = element; node; node = node.parentElement) {
        const overflow = getComputedStyle(node).overflowY;
        if ((overflow === "auto" || overflow === "scroll") && node.scrollHeight > node.clientHeight) {
          node.scrollBy({ top: event.deltaY });
          return;
        }
      }
      return;
    }
  };

  // With a controller: up/down move in the open list, confirm picks, back closes.
  useInputAction(
    (action) => {
      switch (action) {
        case "up":
          pointerActive.current = false;
          setActive((index) => nextEnabled(index, -1));
          return true;
        case "down":
          pointerActive.current = false;
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
      pointerActive.current = false;
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
        ref={trigger}
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

      {open &&
        createPortal(
          <>
            {/* Takes the click that closes the list, so it never reaches what is underneath. */}
            <div
              className={styles.backdrop}
              onWheel={forwardWheel}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                setOpen(false);
              }}
              onContextMenu={(event) => {
                event.preventDefault();
                setOpen(false);
              }}
            />
        <ul ref={list} id={`${id}-list`} role="listbox" className={styles.list} style={place}>
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
              onPointerMove={() => {
                if (option.disabled || index === active) return;
                pointerActive.current = true;
                setActive(index);
              }}
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
          </>,
          document.querySelector("[data-overlay-root]") ?? document.body,
        )}
    </div>
  );
}
