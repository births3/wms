import type { ReactNode } from "react";
import { colors, fontStack } from "../../tokens";

export interface PageHeaderProps {
  title: ReactNode;
  subtitle?: ReactNode;
  /** 右侧操作区（按钮组等） */
  actions?: ReactNode;
  /** 面包屑 */
  breadcrumb?: ReactNode;
}

/**
 * UI 复合 · PageHeader（PC 端管理页统一头部）
 */
export function PageHeader({ title, subtitle, actions, breadcrumb }: PageHeaderProps) {
  return (
    <div
      style={{
        background: "#fff",
        padding: "16px 24px",
        borderBottom: `1px solid ${colors.neutral[200]}`,
        fontFamily: fontStack.sans,
      }}
    >
      {breadcrumb && (
        <div style={{ fontSize: 12, color: colors.neutral[500], marginBottom: 6 }}>{breadcrumb}</div>
      )}
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 18, fontWeight: 600, color: colors.neutral[900] }}>{title}</div>
          {subtitle && (
            <div style={{ fontSize: 12, color: colors.neutral[500], marginTop: 2 }}>{subtitle}</div>
          )}
        </div>
        {actions && <div style={{ display: "flex", gap: 8, alignItems: "center" }}>{actions}</div>}
      </div>
    </div>
  );
}
