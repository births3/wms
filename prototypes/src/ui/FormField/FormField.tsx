import type { ReactNode, CSSProperties } from "react";
import { colors, fontStack } from "../../tokens";

export interface FormFieldProps {
  label?: ReactNode;
  required?: boolean;
  error?: string;
  hint?: string;
  children: ReactNode;
  size?: "sm" | "md" | "lg";
  style?: CSSProperties;
}

/**
 * UI 复合 · FormField（label + control + error/hint）
 */
export function FormField({
  label,
  required,
  error,
  hint,
  children,
  size = "md",
  style,
}: FormFieldProps) {
  const labelFontSize = size === "lg" ? 14 : size === "sm" ? 12 : 13;

  return (
    <div style={{ fontFamily: fontStack.sans, ...style }}>
      {label && (
        <label
          style={{
            fontSize: labelFontSize,
            color: colors.neutral[700],
            fontWeight: 500,
            marginBottom: 6,
            display: "block",
          }}
        >
          {label}
          {required && (
            <span style={{ color: colors.danger, marginLeft: 4 }} aria-label="必填">
              *
            </span>
          )}
        </label>
      )}
      {children}
      {error && (
        <div role="alert" style={{ color: colors.danger, fontSize: labelFontSize - 1, marginTop: 4 }}>
          {error}
        </div>
      )}
      {!error && hint && (
        <div style={{ color: colors.neutral[500], fontSize: labelFontSize - 1, marginTop: 4 }}>
          {hint}
        </div>
      )}
    </div>
  );
}
