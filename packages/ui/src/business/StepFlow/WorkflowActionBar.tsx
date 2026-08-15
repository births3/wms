/**
 * WorkflowActionBar — 单据流转状态条与操作面板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：H7 管理端单据工作流与动作统一能力
 * Wave：Wave 6 管理端增强
 * 业务约束：统一在详情页或单据详情弹窗顶部展示状态机进度条 + 动态可用动作按钮
 *
 * @example
 *   <WorkflowActionBar steps={[{ label: "新建" }, { label: "收货" }]} currentStepIndex={1} actions={[{ key: "receive", label: "收货", onClick: handleReceive }]} />
 */

import * as React from "react";
import { Loader2 } from "lucide-react";
import { cn } from "../../lib/utils";
import { Button, type ButtonProps } from "../../ui/button";
import { StepFlow, type Step } from "./StepFlow";

export interface WorkflowActionItem {
  key: string;
  label: React.ReactNode;
  variant?: ButtonProps["variant"];
  disabled?: boolean;
  loading?: boolean;
  primary?: boolean;
  hidden?: boolean;
  onClick: () => void | Promise<void>;
}

export interface WorkflowActionBarProps extends React.HTMLAttributes<HTMLDivElement> {
  steps: Step[];
  currentStepIndex: number;
  errorSteps?: number[];
  actions?: WorkflowActionItem[];
  extraInfo?: React.ReactNode;
}

export const WorkflowActionBar = React.forwardRef<HTMLDivElement, WorkflowActionBarProps>(
  ({ steps, currentStepIndex, errorSteps, actions = [], extraInfo, className, ...rest }, ref) => {
    const visibleActions = actions.filter((action) => !action.hidden);

    return (
      <div
        ref={ref}
        className={cn(
          "flex flex-col md:flex-row items-start md:items-center justify-between gap-4 rounded-lg border bg-card p-4 shadow-sm font-sans",
          className
        )}
        {...rest}
      >
        {/* 步骤条区域 */}
        <div className="flex-1 min-w-0 w-full md:w-auto">
          <StepFlow
            steps={steps}
            current={currentStepIndex}
            errorSteps={errorSteps}
            size="sm"
            orientation="horizontal"
          />
        </div>

        {/* 状态补充信息与操作按钮 */}
        <div className="flex flex-wrap items-center justify-end gap-2.5 shrink-0 w-full md:w-auto border-t md:border-t-0 pt-3 md:pt-0">
          {extraInfo && <div className="text-xs text-muted-foreground mr-1">{extraInfo}</div>}
          {visibleActions.map((action) => {
            const isPrimary = action.primary ?? (action.variant === "default");
            return (
              <Button
                key={action.key}
                type="button"
                size="sm"
                variant={action.variant ?? (isPrimary ? "default" : "outline")}
                disabled={action.disabled || action.loading}
                onClick={action.onClick}
              >
                {action.loading && <Loader2 className="mr-1.5 size-3.5 animate-spin" aria-hidden />}
                {action.label}
              </Button>
            );
          })}
        </div>
      </div>
    );
  }
);
WorkflowActionBar.displayName = "WorkflowActionBar";

