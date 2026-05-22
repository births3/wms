import type { SelectHTMLAttributes } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface SelectProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  size?: "sm" | "md" | "lg";
  options: { value: string; label: string }[];
  hasError?: boolean;
}

/**
 * UI 原子 · Select
 * Wave 1 替换为 shadcn/ui Select（含搜索、虚拟滚动）
 */
export function Select({
  size = "md",
  options,
  hasError,
  style,
  ...rest
}: SelectProps) {
  const heightMap = { sm: 28, md: 36, lg: 48 };
  const fontSizeMap = { sm: 12, md: 14, lg: 18 };

  return (
    <select
      {...rest}
      style={{
        height: heightMap[size],
        fontSize: fontSizeMap[size],
        padding: "0 12px",
        border: `1px solid ${hasError ? colors.danger : colors.neutral[300]}`,
        borderRadius: radius.md,
        background: rest.disabled ? colors.neutral[100] : "#fff",
        fontFamily: fontStack.sans,
        cursor: "pointer",
        width: "100%",
        boxSizing: "border-box",
        ...style,
      }}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
