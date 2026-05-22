import type { CSSProperties, ReactNode } from "react";
import { colors } from "../../tokens";

export interface FieldRow {
  /** 字段名（左侧标签） */
  label: string;
  /** 字段值（右侧） */
  value: ReactNode;
  /** 是否扫码自动填充（高亮显示） */
  autoFilled?: boolean;
  /** 是否必填（红星） */
  required?: boolean;
  /** 校验失败提示 */
  error?: string;
}

export interface FieldTableProps {
  rows: FieldRow[];
  /** PDA 端使用更大字号 */
  size?: "sm" | "md" | "lg";
  /** 标签列宽度（CSS 值） */
  labelWidth?: string;
  className?: string;
  testId?: string;
}

export function FieldTable({
  rows,
  size = "md",
  labelWidth = "40%",
  className,
  testId,
}: FieldTableProps) {
  const fontSize = size === "sm" ? 14 : size === "lg" ? 18 : 16;
  const padding = size === "lg" ? "12px 16px" : "8px 12px";

  const rowStyle = (autoFilled?: boolean): CSSProperties => ({
    display: "grid",
    gridTemplateColumns: `${labelWidth} 1fr`,
    background: autoFilled ? colors.primaryLight : "transparent",
    borderBottom: `1px solid ${colors.neutral[200]}`,
    transition: "background 0.3s",
  });

  const labelStyle: CSSProperties = {
    padding,
    color: colors.neutral[700],
    fontSize,
    fontWeight: 500,
    background: colors.neutral[50],
    borderRight: `1px solid ${colors.neutral[200]}`,
  };

  const valueStyle: CSSProperties = {
    padding,
    fontSize,
    color: colors.neutral[900],
    wordBreak: "break-all",
  };

  return (
    <div
      data-testid={testId}
      className={className}
      style={{
        border: `1px solid ${colors.neutral[200]}`,
        borderRadius: 6,
        overflow: "hidden",
        background: "#fff",
      }}
    >
      {rows.map((row, i) => (
        <div key={i} style={rowStyle(row.autoFilled)}>
          <div style={labelStyle}>
            {row.label}
            {row.required && (
              <span style={{ color: colors.danger, marginLeft: 4 }} aria-label="必填">
                *
              </span>
            )}
          </div>
          <div style={valueStyle}>
            {row.value}
            {row.error && (
              <div
                role="alert"
                style={{ color: colors.danger, fontSize: fontSize - 2, marginTop: 4 }}
              >
                {row.error}
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
