import type { CSSProperties } from "react";
import { colors } from "../../tokens";

export interface Step {
  /** 步骤标题 */
  label: string;
  /** 可选副标题/说明 */
  description?: string;
}

export interface StepFlowProps {
  steps: Step[];
  /** 当前激活步骤索引（从 0 开始） */
  current: number;
  /** 失败步骤索引集合（红色） */
  errorSteps?: number[];
  /** 布局方向 */
  orientation?: "horizontal" | "vertical";
  /** PDA 端用更大触控目标 */
  size?: "sm" | "md" | "lg";
  className?: string;
  testId?: string;
}

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

  const getStepState = (i: number): "completed" | "current" | "error" | "pending" => {
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

  const containerStyle: CSSProperties = {
    display: "flex",
    flexDirection: isVertical ? "column" : "row",
    gap: 0,
    width: "100%",
  };

  return (
    <div data-testid={testId} className={className} style={containerStyle}>
      {steps.map((step, i) => {
        const state = getStepState(i);
        const color = stateColor[state];
        const isLast = i === steps.length - 1;

        return (
          <div
            key={i}
            data-state={state}
            style={{
              display: "flex",
              flexDirection: isVertical ? "row" : "column",
              alignItems: isVertical ? "flex-start" : "center",
              flex: isVertical ? "0 0 auto" : 1,
              position: "relative",
            }}
          >
            <div
              style={{
                display: "flex",
                flexDirection: isVertical ? "column" : "row",
                alignItems: "center",
                width: isVertical ? "auto" : "100%",
              }}
            >
              {/* 圆点 */}
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
                  zIndex: 1,
                }}
              >
                {state === "completed" ? "✓" : state === "error" ? "✗" : i + 1}
              </div>

              {/* 连接线 */}
              {!isLast && (
                <div
                  style={{
                    flex: 1,
                    height: isVertical ? 24 : 2,
                    width: isVertical ? 2 : "auto",
                    background: i < current ? colors.success : colors.neutral[300],
                    margin: isVertical ? `4px 0 4px ${dotSize / 2 - 1}px` : "0 4px",
                  }}
                />
              )}
            </div>

            {/* 标签 */}
            <div
              style={{
                marginTop: isVertical ? 0 : 8,
                marginLeft: isVertical ? 12 : 0,
                fontSize,
                color: state === "pending" ? colors.neutral[500] : colors.neutral[900],
                fontWeight: state === "current" ? 600 : 400,
                textAlign: isVertical ? "left" : "center",
                lineHeight: 1.4,
              }}
            >
              <div>{step.label}</div>
              {step.description && (
                <div style={{ fontSize: fontSize - 2, color: colors.neutral[500], marginTop: 2 }}>
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
