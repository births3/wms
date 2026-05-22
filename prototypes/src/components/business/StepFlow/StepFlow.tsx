import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Check, X } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * StepFlow — 多步骤流程指示器
 *
 * 层级：Layer 2 业务复合
 * 关联故事：M2-003 PDA 验收（14 步）/ M2-004 双人签字 / M2-005 上架 / M4-003 拣选 / BA-002 批号双签
 * Wave：Wave 1.5 起步（M2 业务页）
 * 业务约束：纵向布局必须保证连接线可见；error 状态用 ✗ 红色
 *
 * @example
 *   <StepFlow current={2} steps={[{ label: "验收" }, { label: "签字" }, { label: "上架" }]} />
 */

export interface Step {
  label: string;
  description?: string;
}

const containerVariants = cva("font-sans", {
  variants: {
    orientation: { horizontal: "flex flex-row w-full", vertical: "flex flex-col" },
  },
  defaultVariants: { orientation: "horizontal" },
});

const sizeMap = {
  sm: { dot: 24, font: 12, gap: 36 },
  default: { dot: 30, font: 14, gap: 44 },
  lg: { dot: 36, font: 16, gap: 56 },
} as const;

export interface StepFlowProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children">,
    VariantProps<typeof containerVariants> {
  steps: Step[];
  current: number;
  errorSteps?: number[];
  size?: "sm" | "default" | "lg";
}

type StepState = "completed" | "current" | "error" | "pending";

export const StepFlow = React.forwardRef<HTMLDivElement, StepFlowProps>(
  ({ steps, current, errorSteps = [], orientation = "horizontal", size = "default", className, ...rest }, ref) => {
    const isVertical = orientation === "vertical";
    const { dot, font, gap } = sizeMap[size];

    const getState = (i: number): StepState => {
      if (errorSteps.includes(i)) return "error";
      if (i < current) return "completed";
      if (i === current) return "current";
      return "pending";
    };

    const stateClass = (s: StepState) => {
      switch (s) {
        case "completed":
          return "bg-wms-success border-wms-success text-white";
        case "current":
          return "bg-primary border-primary text-primary-foreground";
        case "error":
          return "bg-destructive border-destructive text-destructive-foreground";
        case "pending":
          return "bg-background border-border text-muted-foreground";
      }
    };

    const lineClass = (filled: boolean) =>
      filled ? "bg-wms-success" : "bg-border";

    if (isVertical) {
      return (
        <div ref={ref} className={cn(containerVariants({ orientation }), className)} {...rest}>
          {steps.map((step, i) => {
            const state = getState(i);
            const isLast = i === steps.length - 1;
            return (
              <div
                key={i}
                data-state={state}
                className="grid"
                style={{
                  gridTemplateColumns: `${dot}px 1fr`,
                  gridTemplateRows: "auto 1fr",
                  columnGap: 12,
                  minHeight: isLast ? "auto" : gap + dot,
                }}
              >
                {/* dot */}
                <div
                  aria-current={state === "current" ? "step" : undefined}
                  className={cn(
                    "rounded-full border-2 flex items-center justify-center font-semibold col-start-1 row-start-1 z-10",
                    stateClass(state)
                  )}
                  style={{ width: dot, height: dot, fontSize: font - 2 }}
                >
                  {state === "completed" ? <Check className="size-3.5" aria-hidden /> :
                   state === "error" ? <X className="size-3.5" aria-hidden /> : i + 1}
                </div>
                {/* label */}
                <div
                  className="col-start-2 row-start-1"
                  style={{ paddingTop: (dot - font * 1.4) / 2, fontSize: font }}
                >
                  <div
                    className={cn(
                      "leading-tight",
                      state === "pending" ? "text-muted-foreground" : "text-foreground",
                      state === "current" && "font-semibold"
                    )}
                  >
                    {step.label}
                  </div>
                  {step.description && (
                    <div className="text-foreground/70 mt-1" style={{ fontSize: font - 1 }}>
                      {step.description}
                    </div>
                  )}
                </div>
                {/* connector */}
                {!isLast && (
                  <div
                    className={cn("col-start-1 row-start-2 mx-auto", lineClass(i < current))}
                    style={{ width: 2, marginTop: 4, marginBottom: 4, minHeight: gap - 8 }}
                  />
                )}
              </div>
            );
          })}
        </div>
      );
    }

    return (
      <div ref={ref} className={cn(containerVariants({ orientation }), className)} {...rest}>
        {steps.map((step, i) => {
          const state = getState(i);
          const isLast = i === steps.length - 1;
          return (
            <div key={i} data-state={state} className="flex flex-col items-center flex-1">
              <div className="flex items-center w-full">
                <div
                  aria-current={state === "current" ? "step" : undefined}
                  className={cn("rounded-full border-2 flex items-center justify-center font-semibold shrink-0", stateClass(state))}
                  style={{ width: dot, height: dot, fontSize: font - 2 }}
                >
                  {state === "completed" ? <Check className="size-3.5" aria-hidden /> :
                   state === "error" ? <X className="size-3.5" aria-hidden /> : i + 1}
                </div>
                {!isLast && <div className={cn("flex-1 h-0.5 mx-1", lineClass(i < current))} />}
              </div>
              <div className="mt-2 text-center leading-tight" style={{ fontSize: font }}>
                <div className={cn(state === "pending" ? "text-muted-foreground" : "text-foreground", state === "current" && "font-semibold")}>
                  {step.label}
                </div>
                {step.description && (
                  <div className="text-foreground/70 mt-0.5" style={{ fontSize: font - 1 }}>{step.description}</div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    );
  }
);
StepFlow.displayName = "StepFlow";
