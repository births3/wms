import type { InputHTMLAttributes, ReactNode } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface InputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "size"> {
  size?: "sm" | "md" | "lg";
  hasError?: boolean;
  prefix?: ReactNode;
  suffix?: ReactNode;
}

/**
 * UI 原子 · Input
 * Wave 1 替换为 shadcn/ui Input
 */
export function Input({
  size = "md",
  hasError,
  prefix,
  suffix,
  style,
  ...rest
}: InputProps) {
  const heightMap = { sm: 28, md: 36, lg: 48 };
  const fontSizeMap = { sm: 12, md: 14, lg: 18 };

  const wrapperStyle = {
    display: "flex",
    alignItems: "center",
    height: heightMap[size],
    border: `1px solid ${hasError ? colors.danger : colors.neutral[300]}`,
    borderRadius: radius.md,
    background: rest.disabled ? colors.neutral[100] : "#fff",
    overflow: "hidden",
    width: "100%",
    boxSizing: "border-box" as const,
    fontFamily: fontStack.sans,
  };

  return (
    <div style={wrapperStyle}>
      {prefix && <div style={{ padding: "0 8px 0 12px", color: colors.neutral[500], fontSize: fontSizeMap[size] }}>{prefix}</div>}
      <input
        {...rest}
        style={{
          flex: 1,
          height: "100%",
          border: "none",
          outline: "none",
          padding: prefix ? "0 12px 0 4px" : "0 12px",
          fontSize: fontSizeMap[size],
          background: "transparent",
          fontFamily: fontStack.sans,
          ...style,
        }}
      />
      {suffix && <div style={{ padding: "0 12px 0 8px", color: colors.neutral[500], fontSize: fontSizeMap[size] }}>{suffix}</div>}
    </div>
  );
}
