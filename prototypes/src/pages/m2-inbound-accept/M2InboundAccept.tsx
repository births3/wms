import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { useState } from "react";
import { ChevronLeft, AlertTriangle } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  ScanInput,
  StepFlow,
  FieldTable,
  type FieldRow,
} from "@wms/ui";

/**
 * M2InboundAccept — M2-003 PDA 14 步验收
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-003（PDA 14 步验收 / GSP 法定流程 / 含失败步回退）
 * Wave：Wave 1.5（M2 PDA 业务流）
 * 业务约束：14 步严格按 GSP 顺序；任一步失败可回退；外观检查失败需拍照取证
 *
 * @example
 *   <M2InboundAccept />
 */

const STEPS = [
  { label: "扫 ASN", description: "PO-2026-0001" },
  { label: "核对供应商资质", description: "国药控股 ✓" },
  { label: "扫追溯码", description: "AB12-CD34-EF56" },
  { label: "核对品名规格", description: "葡萄糖注射液 500ml" },
  { label: "录入数量", description: "240 瓶" },
  { label: "核对批号效期" },
  { label: "外观检查" },
  { label: "包装检查" },
  { label: "标签检查" },
  { label: "温度检查" },
  { label: "异常拍照" },
  { label: "质量判定" },
  { label: "主管审批" },
  { label: "提交结论" },
];

const FIELD_ROWS: FieldRow[] = [
  { label: "ASN 号", value: "PO-2026-0001", autoFilled: true },
  { label: "供应商", value: "国药控股北京", autoFilled: true },
  { label: "品名", value: "葡萄糖注射液 500ml", autoFilled: true },
  { label: "追溯码", value: "AB12-CD34-EF56", autoFilled: true },
  { label: "实际到货数量", value: "240 瓶", required: true, autoFilled: true },
  { label: "生产批号", value: "20260301A", required: true, autoFilled: true },
  { label: "生产日期", value: "2026-03-01", required: true, autoFilled: true },
  { label: "有效期至", value: "2028-03-01", required: true, autoFilled: true },
];

export function M2InboundAccept() {
  const [scanned, setScanned] = useState<string>("");

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="h-8 w-8">
          <ChevronLeft className="h-5 w-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M2-003 PDA 验收（步骤 6/14）</div>
          <div className="text-sm font-semibold">PO-2026-0001 · 葡萄糖注射液</div>
        </div>
        <StatusBadge status="in_progress" size="sm" label="进行中" />
      </div>

      {/* 步骤指示器（纵向） */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-xs text-muted-foreground mb-2">流程进度（GSP 法定 14 步）</div>
        <StepFlow
          orientation="vertical"
          current={5}
          errorSteps={[]}
          size="sm"
          steps={STEPS.slice(0, 8)}
        />
        <div className="text-xs text-muted-foreground mt-2 italic">+ 6 步未展开</div>
      </div>

      {/* 当前步：扫批号 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="flex items-center justify-between mb-2">
          <div className="text-sm font-semibold">⑥ 核对批号效期</div>
          <span className="text-xs px-2 py-0.5 bg-primary/10 text-primary rounded font-medium">当前</span>
        </div>
        <div className="text-xs text-muted-foreground mb-3">
          扫描包装批号或手动输入，与采购单匹配后自动填充效期
        </div>
        <ScanInput
          mode="scanner"
          placeholder="扫描批号或输入"
          onScan={(code) => setScanned(code)}
          lastScanned={scanned}
        />
      </div>

      {/* 已采集字段 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <Card className="p-3">
          <div className="text-sm font-semibold mb-2">已采集信息</div>
          <FieldTable size="sm" rows={FIELD_ROWS} />
        </Card>

        {/* 异常提示 */}
        <div className="mt-3 flex items-start gap-2 p-3 bg-wms-warning/10 rounded-md border border-wms-warning/30">
          <AlertTriangle className="h-4 w-4 text-wms-warning flex-shrink-0 mt-0.5" />
          <div className="text-xs">
            <div className="font-medium text-wms-warning">效期 &lt; 12 个月</div>
            <div className="text-muted-foreground mt-0.5">需双人复核签字，自动跳转 M2-004</div>
          </div>
        </div>
      </div>

      {/* 底部操作栏 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">回退</Button>
        <Button variant="outline" className="flex-1 h-10">异常拍照</Button>
        <Button className="flex-1 h-10">下一步 →</Button>
      </div>
    </div>
  );
}
