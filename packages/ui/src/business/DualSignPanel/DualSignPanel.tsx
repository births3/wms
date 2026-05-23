import * as React from "react";
import { cn } from "../../lib/utils";
import { StatusBadge } from "../StatusBadge";
import { Check, Clock, UserCheck } from "lucide-react";

/**
 * DualSignPanel — 双人签字面板
 *
 * 层级：Layer 2 业务复合
 * 关联故事：US-M2-004（入库双人验收）/ US-VR-006（双人策略矩阵）/ US-BA-002（批号调整双签）
 * Wave：Wave 1.5（M2 业务页）
 * 业务约束：第一人 ≠ 第二人（user_id 校验）；签字记录 append-only；策略档位（single/dual_scan/dual_scan_with_approval）
 *
 * @example
 *   <DualSignPanel policy="dual_scan_with_approval" first={{user:"u001",time:"09:14"}} />
 */

export type DualSignPolicy = "single" | "dual_scan" | "dual_scan_with_approval";

export interface DualSignSlot {
  user: string;
  time: string;
  comment?: string;
}

export interface DualSignPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  policy: DualSignPolicy;
  /** 第一人（已签字时填，否则 undefined） */
  first?: DualSignSlot;
  /** 第二人（policy=single 时不展示） */
  second?: DualSignSlot;
  /** 主管审批（仅 dual_scan_with_approval） */
  approval?: DualSignSlot;
}

const POLICY_LABEL: Record<DualSignPolicy, string> = {
  single: "单人签字",
  dual_scan: "双人签字",
  dual_scan_with_approval: "双人签字 + 主管审批",
};

export const DualSignPanel = React.forwardRef<HTMLDivElement, DualSignPanelProps>(
  ({ policy, first, second, approval, className, ...rest }, ref) => {
    const slots: { label: string; slot?: DualSignSlot; show: boolean }[] = [
      { label: "第一人签字", slot: first, show: true },
      { label: "第二人签字", slot: second, show: policy !== "single" },
      { label: "主管审批", slot: approval, show: policy === "dual_scan_with_approval" },
    ].filter((s) => s.show);

    return (
      <div ref={ref} className={cn("space-y-3 font-sans", className)} {...rest}>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <UserCheck className="size-3.5" />
          <span>签字策略：{POLICY_LABEL[policy]}</span>
        </div>
        <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${slots.length}, 1fr)` }}>
          {slots.map((s, i) => (
            <Slot key={i} label={s.label} slot={s.slot} />
          ))}
        </div>
      </div>
    );
  }
);
DualSignPanel.displayName = "DualSignPanel";

function Slot({ label, slot }: { label: string; slot?: DualSignSlot }) {
  const signed = !!slot;
  return (
    <div
      className={cn(
        "rounded-md border-2 p-4 transition-colors",
        signed ? "border-wms-success bg-wms-success/5" : "border-dashed border-muted-foreground/30 bg-muted/30"
      )}
    >
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">{label}</span>
        {signed ? (
          <StatusBadge status="completed" size="sm" label="已签" />
        ) : (
          <StatusBadge status="pending" size="sm" label="待签" />
        )}
      </div>
      {signed && slot ? (
        <div className="mt-2 space-y-1">
          <div className="flex items-center gap-1.5 text-sm font-medium">
            <Check className="size-3.5 text-wms-success" />
            {slot.user}
          </div>
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Clock className="size-3" />
            {slot.time}
          </div>
          {slot.comment && (
            <div className="text-xs text-foreground/70 mt-1 italic">"{slot.comment}"</div>
          )}
        </div>
      ) : (
        <div className="mt-2 text-xs text-muted-foreground">等待签字...</div>
      )}
    </div>
  );
}
