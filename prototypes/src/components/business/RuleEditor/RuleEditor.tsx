import * as React from "react";
import { cn } from "@/lib/utils";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";

/**
 * RuleEditor — 校验规则配置编辑器（条件组合 AND/OR + 动作）
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-VR-001（校验规则）/ US-MPM-002（参数映射）/ US-M9-001（计费规则）/ US-AL-001（告警规则）
 * Wave：Wave 3（M-VR/M-PM 业务页）
 * 业务约束：条件组之间默认 OR；组内默认 AND；动作可叠加
 *
 * @example
 *   <RuleEditor groups={[...]} actions={[...]} onChange={...} />
 */

export interface RuleCondition {
  field: string;
  op: "eq" | "neq" | "gt" | "lt" | "gte" | "lte" | "in" | "contains";
  value: string;
}

export interface RuleGroup {
  /** 组内连接符（默认 AND） */
  connector?: "AND" | "OR";
  conditions: RuleCondition[];
}

export interface RuleAction {
  type: string;
  label: string;
  /** 显示参数（key:value） */
  params?: Record<string, string>;
}

export interface RuleEditorProps extends React.HTMLAttributes<HTMLDivElement> {
  groups: RuleGroup[];
  actions: RuleAction[];
  /** 字段候选（用于条件 dropdown，仅展示） */
  fields?: string[];
  /** 只读模式 */
  readOnly?: boolean;
}

const OP_LABEL: Record<RuleCondition["op"], string> = {
  eq: "=",
  neq: "≠",
  gt: ">",
  lt: "<",
  gte: "≥",
  lte: "≤",
  in: "IN",
  contains: "包含",
};

export const RuleEditor = React.forwardRef<HTMLDivElement, RuleEditorProps>(
  ({ groups, actions, readOnly, className, ...rest }, ref) => {
    return (
      <div ref={ref} className={cn("space-y-4 font-sans", className)} {...rest}>
        {/* 条件区 */}
        <div>
          <div className="text-xs text-muted-foreground mb-2 font-medium uppercase tracking-wide">条件（IF）</div>
          <div className="space-y-2">
            {groups.map((g, gi) => (
              <React.Fragment key={gi}>
                <div className="rounded-md border bg-background p-3">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs text-muted-foreground">组 {gi + 1}（内部 {g.connector ?? "AND"}）</span>
                    {!readOnly && (
                      <Button variant="ghost" size="sm" className="h-6 px-2 text-muted-foreground">
                        <Trash2 className="size-3" />
                      </Button>
                    )}
                  </div>
                  <div className="space-y-1.5">
                    {g.conditions.map((c, ci) => (
                      <div key={ci} className="flex items-center gap-2 text-sm">
                        {ci > 0 && (
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground font-medium">
                            {g.connector ?? "AND"}
                          </span>
                        )}
                        <code className="px-2 py-1 bg-muted rounded text-xs font-mono">{c.field}</code>
                        <span className="text-xs font-bold text-primary">{OP_LABEL[c.op]}</span>
                        <code className="px-2 py-1 bg-primary/10 rounded text-xs font-mono">{c.value}</code>
                      </div>
                    ))}
                    {!readOnly && (
                      <Button variant="ghost" size="sm" className="h-7 text-xs">
                        <Plus className="size-3" />
                        添加条件
                      </Button>
                    )}
                  </div>
                </div>
                {gi < groups.length - 1 && (
                  <div className="flex items-center justify-center">
                    <span className="text-xs px-2.5 py-0.5 rounded-full bg-foreground/80 text-background font-medium">OR</span>
                  </div>
                )}
              </React.Fragment>
            ))}
            {!readOnly && (
              <Button variant="outline" size="sm" className="w-full">
                <Plus className="size-3.5" />
                添加 OR 组
              </Button>
            )}
          </div>
        </div>

        {/* 动作区 */}
        <div>
          <div className="text-xs text-muted-foreground mb-2 font-medium uppercase tracking-wide">动作（THEN）</div>
          <div className="space-y-2">
            {actions.map((a, i) => (
              <div key={i} className="rounded-md border bg-background p-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">{a.label}</span>
                  <code className="text-xs text-muted-foreground font-mono">{a.type}</code>
                </div>
                {a.params && Object.keys(a.params).length > 0 && (
                  <div className="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
                    {Object.entries(a.params).map(([k, v]) => (
                      <div key={k} className="flex items-center gap-2">
                        <span className="text-muted-foreground">{k}</span>
                        <code className="font-mono">{v}</code>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
            {!readOnly && (
              <Button variant="outline" size="sm" className="w-full">
                <Plus className="size-3.5" />
                添加动作
              </Button>
            )}
          </div>
        </div>
      </div>
    );
  }
);
RuleEditor.displayName = "RuleEditor";
