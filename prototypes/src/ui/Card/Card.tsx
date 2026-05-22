import type { CSSProperties, ReactNode } from "react";
import { colors, radius } from "../../tokens";

export interface CardProps {
  children?: ReactNode;
  /** 内边距 */
  padding?: "none" | "sm" | "md" | "lg";
  /** 是否有边框（默认 true） */
  bordered?: boolean;
  /** 阴影层级 */
  elevation?: "none" | "sm" | "md" | "lg";
  className?: string;
  style?: CSSProperties;
}

const PADDING_MAP = { none: 0, sm: 12, md: 20, lg: 28 };

const SHADOW_MAP = {
  none: "none",
  sm: "0 1px 2px rgba(0,0,0,0.05)",
  md: "0 4px 12px rgba(0,0,0,0.08)",
  lg: "0 20px 60px rgba(0,0,0,0.20)",
};

/**
 * UI 原子 · Card
 * Wave 1 替换为 shadcn/ui Card
 */
export function Card({
  children,
  padding = "md",
  bordered = true,
  elevation = "none",
  className,
  style,
}: CardProps) {
  return (
    <div
      className={className}
      style={{
        background: "#fff",
        borderRadius: radius.lg,
        border: bordered ? `1px solid ${colors.neutral[200]}` : "none",
        boxShadow: SHADOW_MAP[elevation],
        padding: PADDING_MAP[padding],
        ...style,
      }}
    >
      {children}
    </div>
  );
}
