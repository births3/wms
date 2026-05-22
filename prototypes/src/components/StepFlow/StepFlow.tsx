import type { CSSProperties } from "react";
import { colors, radius, fontStack } from "../../tokens";

export interface Step {
  label: string;
  description?: string;
}

export interface StepFlowProps {
  steps: Step[];
  current: number;
  errorSteps?: number[];
  orientation?: "horizontal" | "vertical";
  size?: "sm" | "md" | "lg";
  className?: string;
  testId?: string;
}

type StepState = "completed" | "current" | "error" | "pending";

export function StepFlow({
  steps,
  current,
  errorSteps = [],
  orientation = "horizontal",
  size = "md",
  className,
  testId,
}: StepFlowProps) {
  const isVertical = orientation === "vertical";
  const dotSize = size === "lg" ? 36 : size === "sm" ? 24 : 30;
  const fontSize = size === "lg" ? 16 : size === "sm" ? 12 : 14;
  // 纵向 step 之间的最小间距（保证连接线可见）
  const minGap = size === "lg" ? 56 : size === "sm" ? 36 : 44;

  const getStepState = (i: number): StepState => {
    if (errorSteps.includes(i)) return "error";
    if (i < current) return "completed";
    if (i === current) return "current";
    return "pending";
  };

  const stateColor = {
    completed: colors.success,
    current: colors.primary,
    error: colors.danger,
    pending: colors.neutral[300],
  };

  if (isVertical) {
    return (
      <div
        data-testid={testId}
        className={className}
        style={{ display: "flex", flexDirection: "column", fontFamily: fontStack.sans }}
      >
        {steps.map((step, i) => {
          const state = getStepState(i);
          const color = stateColor[state];
          const isLast = i === steps.length - 1;
          const lineColor = i < current ? colors.success : colors.neutral[300];

          return (
            <div
              key={i}
              data-state={state}
              style={{
                display: "grid",
                gridTemplateColumns: `${dotSize}px 1fr`,
                gridTemplateRows: "auto 1fr",
                columnGap: 12,
                rowGap: 0,
                minHeight: isLast ? "auto" : minGap + dotSize,
              }}
            >
              {/* dot */}
              <div
                aria-current={state === "current" ? "step" : undefined}
                style={{
                  width: dotSize,
                  height: dotSize,
                  borderRadius: "50%",
                  background: state === "pending" ? "#fff" : color,
                  border: `2px solid ${color}`,
                  color: state === "pending" ? colors.neutral[500] : "#fff",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: fontSize - 2,
                  fontWeight: 600,
                  gridColumn: 1,
                  gridRow: 1,
                }}
              >
                {state === "completed" ? "✓" : state === "error" ? "✗" : i + 1}
              </div>
              {/* label */}
              <div
                style={{
                  gridColumn: 2,
                  gridRow: 1,
                  fontSize,
                  color: state === "pending" ? colors.neutral[500] : colors.neutral[900],
                  fontWeight: state === "current" ? 600 : 400,
                  lineHeight: 1.4,
                  paddingTop: (dotSize - fontSize * 1.4) / 2,
                }}
              >
                <div>{step.label}</div>
                {step.description && (
                  <div style={{ fontSize: fontSize - 1, color: colors.neutral[700], marginTop: 4 }}>
                    {step.description}
                  </div>
                )}
              </div>
              {/* connector line */}
              {!isLast && (
                <div
                  style={{
                    gridColumn: 1,
                    gridRow: 2,
                    width: 2,
                    background: lineColor,
                    margin: "4px auto",
                    minHeight: minGap - 8,
                  }}
                />
              )}
            </div>
          );
        })}
      </div>
    );
  }

  // horizontal
  return (
    <div
      data-testid={testId}
      className={className}
      style={{ display: "flex", flexDirection: "row", fontFamily: fontStack.sans, width: "100%" }}
    >
      {steps.map((step, i) => {
        const state = getStepState(i);
        const color = stateColor[state];
        const isLast = i === steps.length - 1;

        return (
          <div
            key={i}
            data-state={state}
            style={{ display: "flex", flexDirection: "column", alignItems: "center", flex: 1 }}
          >
            <div style={{ display: "flex", flexDirection: "row", alignItems: "center", width: "100%" }}>
              <div
                aria-current={state === "current" ? "step" : undefined}
                style={{
                  width: dotSize,
                  height: dotSize,
                  borderRadius: "50%",
                  background: state === "pending" ? "#fff" : color,
                  border: `2px solid ${color}`,
                  color: state === "pending" ? colors.neutral[500] : "#fff",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: fontSize - 2,
                  fontWeight: 600,
                  flexShrink: 0,
                }}
              >
                {state === "completed" ? "✓" : state === "error" ? "✗" : i + 1}
              </div>
              {!isLast && (
                <div
                  style={{
                    flex: 1,
                    height: 2,
                    background: i < current ? colors.success : colors.neutral[300],
                    margin: "0 4px",
                  }}
                />
              )}
            </div>
            <div
              style={{
                marginTop: 8,
                fontSize,
                color: state === "pending" ? colors.neutral[500] : colors.neutral[900],
                fontWeight: state === "current" ? 600 : 400,
                textAlign: "center",
                lineHeight: 1.4,
              }}
            >
              <div>{step.label}</div>
              {step.description && (
                <div style={{ fontSize: fontSize - 1, color: colors.neutral[700], marginTop: 2 }}>
                  {step.description}
                </div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export { radius as _r };
