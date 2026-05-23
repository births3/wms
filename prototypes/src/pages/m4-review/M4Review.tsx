import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { useState } from "react";
import { ChevronLeft, CheckCircle2 } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  ScanInput,
  FieldTable,
  DiffPanel,
  type FieldRow,
} from "@wms/ui";

/**
 * M4Review — M4-004 PDA 复核
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M4-004（PDA 出库复核 / 扫码核对 / 数量批号一致性）
 * Wave：Wave 2.5（M4 出库 PDA 业务流）
 * 业务约束：必须扫追溯码核对；与拣货记录差异需登记原因；麻醉药双人复核
 *
 * @example
 *   <M4Review />
 */

const REVIEW_FIELDS: FieldRow[] = [
  { label: "客户", value: "北京同仁堂连锁药店", autoFilled: true },
  { label: "SO", value: "SO-2026-0042", autoFilled: true },
  { label: "拣货员", value: "王五 (u003)", autoFilled: true },
  { label: "本行品名", value: "葡萄糖注射液 500ml", autoFilled: true },
  { label: "扫描追溯码", value: "AB12-CD34-EF56", required: true, autoFilled: true },
  { label: "实拣数量", value: "5 瓶", autoFilled: true },
  { label: "复核数量", value: "5 瓶", required: true, autoFilled: true },
];

export function M4Review() {
  const [scanned, setScanned] = useState<string>("AB12-CD34-EF56");

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="h-8 w-8">
          <ChevronLeft className="h-5 w-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M4-004 PDA 复核 · 第 1 行 / 共 11 行</div>
          <div className="text-sm font-semibold">SO-2026-0042</div>
        </div>
        <StatusBadge status="in_progress" size="sm" label="复核中" />
      </div>

      {/* 扫描区 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-sm font-semibold mb-2">① 扫追溯码</div>
        <ScanInput
          mode="scanner"
          placeholder="扫描包装追溯码"
          onScan={(code) => setScanned(code)}
          lastScanned={scanned}
        />
        {scanned && (
          <div className="mt-2 flex items-center gap-2 text-xs text-wms-success">
            <CheckCircle2 className="h-4 w-4" />
            <span>追溯码匹配 · 与拣货记录一致</span>
          </div>
        )}
      </div>

      {/* 字段核对 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-sm font-semibold mb-2">② 信息核对</div>
        <FieldTable size="sm" rows={REVIEW_FIELDS} />
      </div>

      {/* 与拣货记录对比 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <Card className="p-3">
          <div className="text-sm font-semibold mb-2">③ 与拣货记录对比</div>
          <DiffPanel
            layout="stacked"
            highlightChanged
            before={{
              "拣货员": "王五 (u003)",
              "拣货时间": "10:15:42",
              "数量": "5 瓶",
              "批号": "20250901A",
            }}
            after={{
              "复核员": "赵六 (u004)",
              "复核时间": "10:32:18",
              "数量": "5 瓶（一致）",
              "批号": "20250901A（一致）",
            }}
          />
          <div className="mt-3 flex items-center gap-2 text-xs text-wms-success">
            <CheckCircle2 className="h-4 w-4" />
            <span>本行复核通过 · 无差异</span>
          </div>
        </Card>

        {/* 进度提示 */}
        <Card className="p-3 mt-3 bg-background">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">本单复核进度</span>
            <span className="font-semibold">1 / 11 行</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden mt-2">
            <div className="h-full bg-primary" style={{ width: "9%" }} />
          </div>
        </Card>
      </div>

      {/* 底部 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">登记差异</Button>
        <Button variant="outline" className="flex-1 h-10">回退</Button>
        <Button className="flex-[2] h-10">下一行 →</Button>
      </div>
    </div>
  );
}
