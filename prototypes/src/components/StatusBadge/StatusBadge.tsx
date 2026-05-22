import type { CSSProperties, ReactNode } from "react";
import { colors } from "../../tokens";

/** 状态枚举（对齐 docs/prototypes/component-registry.md §4.3） */
export type StatusKey =
  | "qualified"
  | "unqualified"
  | "pending"
  | "isolated"
  | "expired"
  | "near_expiry"
  | "in_progress"
  | "completed"
  | "offline_cached";

interface StatusMeta {
  bg: string;
  fg: string;
  text: string;
  icon: ReactNode;
}

const STATUS_MAP: Record<StatusKey, StatusMeta> = {
  qualified: { bg: colors.success, fg: "#fff", text: "合格", icon: "✓" },
  unqualified: { bg: colors.danger, fg: "#fff", text: "不合格", icon: "✗" },
  pending: { bg: colors.warning, fg: "#fff", text: "待处理", icon: "⏳" },
  isolated: { bg: colors.neutral[500], fg: "#fff", text: "隔离", icon: "🔒" },
  expired: { bg: colors.danger, fg: "#fff", text: "已过期", icon: "⚠" },
  near_expiry: { bg: colors.warning, fg: "#fff", text: "近效期", icon: "⚠" },
  in_progress: { bg: colors.primary, fg: "#fff", text: "进行中", icon: "→" },
  completed: { bg: colors.success, fg: "#fff", text: "已完成", icon: "✓" },
  offline_cached: { bg: colors.neutral[400], fg: "#fff", text: "离线暂存", icon: "☁" },
};

export interface StatusBadgeProps {
  status: StatusKey;
  /** PDA 端用更大尺寸（44pt 触控可读） */
  size?: "sm" | "md" | "lg";
  /** 自定义文字（覆盖默认） */
  label?: string;
  className?: string;
  testId?: string;
}

export function StatusBadge({
  status,
  size = "md",
  label,
  className,
  testId,
}: StatusBadgeProps) {
  const meta = STATUS_MAP[status];
  const sizeStyle: Record<string, CSSProperties> = {
    sm: { fontSize: 12, padding: "2px 8px", gap: 4 },
    md: { fontSize: 14, padding: "4px 10px", gap: 6 },
    lg: { fontSize: 18, padding: "6px 14px", gap: 8 }, // PDA 推荐
  };

  return (
    <span
      data-testid={testId}
      data-status={status}
      className={className}
      style={{
        display: "inline-flex",
        alignItems: "center",
        background: meta.bg,
        color: meta.fg,
        borderRadius: 6,
        fontWeight: 500,
        whiteSpace: "nowrap",
        ...sizeStyle[size],
      }}
    >
      <span aria-hidden>{meta.icon}</span>
      <span>{label ?? meta.text}</span>
    </span>
  );
}
