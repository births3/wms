import type { ReactNode } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface TabItem<T extends string = string> {
  value: T;
  label: ReactNode;
}

export interface TabsProps<T extends string = string> {
  items: TabItem<T>[];
  value: T;
  onChange: (value: T) => void;
  /** segment = 圆角分段控件（PDA 友好）/ underline = 下划线（PC 工作台） */
  variant?: "segment" | "underline";
  size?: "sm" | "md" | "lg";
}

/**
 * UI 复合 · Tabs
 * Wave 1 替换为 shadcn/ui Tabs
 */
export function Tabs<T extends string = string>({
  items,
  value,
  onChange,
  variant = "segment",
  size = "md",
}: TabsProps<T>) {
  const heightMap = { sm: 32, md: 40, lg: 48 };
  const fontSizeMap = { sm: 12, md: 14, lg: 16 };

  if (variant === "segment") {
    return (
      <div
        role="tablist"
        style={{
          display: "flex",
          border: `1px solid ${colors.neutral[300]}`,
          borderRadius: radius.md,
          overflow: "hidden",
          fontFamily: fontStack.sans,
        }}
      >
        {items.map((item, i) => {
          const active = item.value === value;
          return (
            <button
              key={item.value}
              role="tab"
              aria-selected={active}
              onClick={() => onChange(item.value)}
              style={{
                flex: 1,
                height: heightMap[size],
                fontSize: fontSizeMap[size],
                background: active ? colors.primary : "#fff",
                color: active ? "#fff" : colors.neutral[700],
                border: "none",
                borderLeft: i > 0 ? `1px solid ${colors.neutral[300]}` : "none",
                fontWeight: active ? 600 : 400,
                cursor: "pointer",
                fontFamily: fontStack.sans,
              }}
            >
              {item.label}
            </button>
          );
        })}
      </div>
    );
  }

  // underline
  return (
    <div
      role="tablist"
      style={{
        display: "flex",
        borderBottom: `1px solid ${colors.neutral[200]}`,
        gap: 24,
        fontFamily: fontStack.sans,
      }}
    >
      {items.map((item) => {
        const active = item.value === value;
        return (
          <button
            key={item.value}
            role="tab"
            aria-selected={active}
            onClick={() => onChange(item.value)}
            style={{
              height: heightMap[size],
              fontSize: fontSizeMap[size],
              background: "transparent",
              color: active ? colors.primary : colors.neutral[600],
              border: "none",
              borderBottom: active ? `2px solid ${colors.primary}` : "2px solid transparent",
              fontWeight: active ? 600 : 400,
              cursor: "pointer",
              padding: "0 4px",
              marginBottom: -1,
            }}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
