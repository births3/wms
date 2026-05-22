import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  StatusBadge,
  OfflineIndicator,
  StepFlow,
  DualSignPanel,
  FieldTable,
} from "@/components/business";

/**
 * M2DualSign — M2-004 PDA 双人验收签字页
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-004（双人验收签字 / dual_scan_with_approval 三档策略）
 * Wave：Wave 1.5（M2 业务页）
 * 业务约束：第一人 ≠ 第二人；策略由 M-VR 矩阵动态查询；签字 append-only
 *
 * @example
 *   <M2DualSign />
 */
export function M2DualSign() {
  return (
    <div data-device="pda" className="w-[480px] min-h-[800px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶部 ASN 信息 */}
      <div className="bg-background px-4 py-3 border-b">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs text-muted-foreground">M2-004 双人验收签字</div>
            <div className="text-base font-semibold mt-0.5">ASN PO-2026-0001</div>
          </div>
          <StatusBadge status="pending" size="default" label="待第二人" />
        </div>
        <div className="mt-2 text-xs text-muted-foreground">
          葡萄糖注射液 500ml × 24 瓶 · 批号 20260301A · 240 瓶
        </div>
      </div>

      {/* 进度指示器 */}
      <div className="bg-background px-4 py-4 border-b">
        <StepFlow
          current={1}
          steps={[
            { label: "第一人签字", description: "张三 ✓" },
            { label: "第二人签字", description: "签字中" },
            { label: "主管审批" },
            { label: "上架中" },
          ]}
        />
      </div>

      {/* 双人签字面板 */}
      <Card className="m-3 p-4 rounded-lg">
        <div className="text-sm font-medium mb-3">签字状态</div>
        <DualSignPanel
          policy="dual_scan_with_approval"
          first={{
            user: "张三 (u001)",
            time: "2026-05-22 09:14:23",
            comment: "外观完好，数量核对一致",
          }}
        />
      </Card>

      {/* 验收摘要 */}
      <Card className="mx-3 p-0 overflow-hidden">
        <div className="px-4 py-2 text-xs font-medium border-b bg-muted/40">验收摘要</div>
        <FieldTable
          size="sm"
          rows={[
            { label: "品名", value: "葡萄糖注射液", autoFilled: true },
            { label: "批号", value: "20260301A", autoFilled: true },
            { label: "实际到货", value: "240 瓶" },
            {
              label: "外观/包装",
              value: <StatusBadge status="qualified" size="sm" />,
            },
            {
              label: "标签",
              value: <StatusBadge status="qualified" size="sm" />,
            },
            {
              label: "第一人结论",
              value: <StatusBadge status="completed" size="sm" label="合格" />,
            },
          ]}
        />
      </Card>

      {/* 底部操作 */}
      <div className="flex-1" />
      <div className="bg-background border-t px-4 py-3 space-y-2">
        <p className="text-xs text-muted-foreground text-center">
          请第二人扫工牌登录后点击"签字确认"
        </p>
        <div className="flex gap-2">
          <Button variant="outline" className="flex-1 h-12">驳回</Button>
          <Button className="flex-1 h-12">签字确认</Button>
        </div>
      </div>
    </div>
  );
}
