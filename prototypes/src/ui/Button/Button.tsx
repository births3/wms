import type { ButtonHTMLAttributes, CSSProperties, ReactNode } from "react";
import { colors, radius, fontStack } from "../../tokens";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger" | "link";
export type ButtonSize = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  block?: boolean;
  iconBefore?: ReactNode;
  iconAfter?: ReactNode;
}

/**
 * UI 原子 · Button
 * 注：原型用最小实现。Wave 1 真做时替换为 shadcn/ui Button（ADR-0021 §2 Layer 1）
 */
export function Button({
  variant = "primary",
  size = "md",
  block,
  iconBefore,
  iconAfter,
  children,
  style,
  ...rest
}: ButtonProps) {
  const sizeMap: Record<ButtonSize, CSSProperties> = {
    sm: { height: 28, padding: "0 12px", fontSize: 12 },
    md: { height: 36, padding: "0 16px", fontSize: 14 },
    lg: { height: 48, padding: "0 24px", fontSize: 16 },
  };

  const variantMap: Record<ButtonVariant, CSSProperties> = {
    primary: { background: colors.primary, color: "#fff", border: "none" },
    secondary: { background: "#fff", color: colors.neutral[700], border: `1px solid ${colors.neutral[300]}` },
    ghost: { background: "transparent", color: colors.neutral[700], border: "none" },
    danger: { background: colors.danger, color: "#fff", border: "none" },
    link: { background: "transparent", color: colors.primary, border: "none", padding: 0, height: "auto" },
  };

  return (
    <button
      {...rest}
      style={{
        ...sizeMap[size],
        ...variantMap[variant],
        width: block ? "100%" : undefined,
        borderRadius: radius.md,
        cursor: rest.disabled ? "not-allowed" : "pointer",
        opacity: rest.disabled ? 0.5 : 1,
        fontWeight: 500,
        fontFamily: fontStack.sans,
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: 6,
        whiteSpace: "nowrap",
        ...style,
      }}
    >
      {iconBefore}
      {children}
      {iconAfter}
    </button>
  );
}
