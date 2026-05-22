import type { CSSProperties, ReactNode } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface FieldRow {
  label: string;
  value: ReactNode;
  autoFilled?: boolean;
  required?: boolean;
  error?: string;
}

export interface FieldTableProps {
  rows: FieldRow[];
  size?: "sm" | "md" | "lg";
  /** 标签列宽度；不传时按 size 自适应（sm=35%, md=40%, lg=46%） */
  labelWidth?: string;
  className?: string;
  testId?: string;
}

export function FieldTable({
  rows,
  size = "md",
  labelWidth,
  className,
  testId,
}: FieldTableProps) {
  const fontSize = size === "sm" ? 14 : size === "lg" ? 18 : 16;
  const padding = size === "lg" ? "12px 16px" : "8px 12px";
  const resolvedLabelWidth =
    labelWidth ?? (size === "lg" ? "46%" : size === "sm" ? "35%" : "40%");

  const rowStyle = (autoFilled?: boolean): CSSProperties => ({
    display: "grid",
    gridTemplateColumns: `${resolvedLabelWidth} 1fr`,
    background: autoFilled ? "#E0EBFF" : "transparent", // 加深 autoFilled 背景
    borderLeft: autoFilled ? `4px solid ${colors.primary}` : `4px solid transparent`,
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
    display: "flex",
    alignItems: "center",
  };

  const valueStyle: CSSProperties = {
    padding,
    fontSize,
    color: colors.neutral[900],
    wordBreak: "break-all",
    display: "flex",
    flexDirection: "column",
    justifyContent: "center",
  };

  return (
    <div
      data-testid={testId}
      className={className}
      style={{
        border: `1px solid ${colors.neutral[200]}`,
        borderRadius: radius.md,
        overflow: "hidden",
        background: "#fff",
        fontFamily: fontStack.sans,
      }}
    >
      {rows.map((row, i) => (
        <div key={i} style={rowStyle(row.autoFilled)}>
          <div style={labelStyle}>
            <span>
              {row.label}
              {row.required && (
                <span style={{ color: colors.danger, marginLeft: 4 }} aria-label="必填">
                  *
                </span>
              )}
            </span>
          </div>
          <div style={valueStyle}>
            <div>{row.value}</div>
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
