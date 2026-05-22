import type { InputHTMLAttributes, ReactNode } from "react";
import { colors, fontStack } from "../../tokens";

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> {
  label?: ReactNode;
  size?: "sm" | "md";
}

/**
 * UI 原子 · Checkbox
 * Wave 1 替换为 shadcn/ui Checkbox
 */
export function Checkbox({ label, size = "md", style, ...rest }: CheckboxProps) {
  const fontSize = size === "sm" ? 12 : 14;
  return (
    <label
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
        fontSize,
        color: rest.disabled ? colors.neutral[400] : colors.neutral[700],
        cursor: rest.disabled ? "not-allowed" : "pointer",
        fontFamily: fontStack.sans,
        ...style,
      }}
    >
      <input type="checkbox" {...rest} style={{ cursor: "inherit" }} />
      {label}
    </label>
  );
}
