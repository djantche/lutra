import type { ButtonHTMLAttributes } from "react";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./Button.module.scss";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "glass";
  size?: "md" | "lg";
  icon?: IconName;
}

export function Button({ variant = "secondary", size = "md", icon, className, children, ...rest }: ButtonProps) {
  return (
    <button
      type="button"
      data-focusable
      className={cx(styles.button, styles[variant], styles[size], !children && styles.iconOnly, className)}
      {...rest}
    >
      {icon && <Icon name={icon} size={size === "lg" ? 20 : 18} />}
      {children}
    </button>
  );
}
