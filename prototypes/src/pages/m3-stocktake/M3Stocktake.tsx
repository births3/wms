import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { useState } from "react";
import { ChevronLeft, MapPin } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  ScanInput,
  StepFlow,
  DiffPanel,
  FieldTable,
  type FieldRow,
} from "@wms/ui";

/**
 * M3Stocktake — M3-005 PDA 盘点
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M3-005（PDA 盘点 / 按库位 / 盈亏对比 / 双盘 / 异常调账）
 * Wave：Wave 2.5（M3 库存核心）
 * 业务约束：必须双盘（同库位两人独立盘）；盈亏 > 1% 触发主管审核；麻醉药每月必盘
 *
 * @example
 *   <M3Stocktake />
 */

const FIELDS: FieldRow[] = [
  { label: "盘点单", value: "ST-2026-0511", autoFilled: true },
  { label: "库位", value: "A-01-02-03", autoFilled: true },
  { label: "品名", value: "葡萄糖注射液 500ml", autoFilled: true },
  { label: "批号", value: "20250901A", autoFilled: true },
  { label: "账面数量", value: "480 瓶", autoFilled: true },
  { label: "实盘数量", value: "478 瓶", required: true, error: "差异 -2 瓶" },
];

export function M3Stocktake() {
  const [scanned, setScanned] = useState<string>("AB12-CD34-EF56");

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="size-8">
          <ChevronLeft className="size-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M3-005 PDA 盘点 · 第 8 / 24 库位</div>
          <div className="text-sm font-semibold">ST-2026-0511 · A 区</div>
        </div>
        <StatusBadge status="in_progress" size="sm" label="第二盘" />
      </div>

      {/* 进度 */}
      <div className="bg-background px-3 py-3 border-b">
        <StepFlow
          current={2}
          size="sm"
          steps={[
            { label: "扫库位", description: "A-01-02-03 ✓" },
            { label: "扫货核对", description: "盘点中" },
            { label: "差异确认" },
            { label: "提交本位" },
          ]}
        />
      </div>

      {/* 当前库位 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="flex items-center gap-2 mb-3">
          <MapPin className="size-5 text-primary" />
          <div className="font-mono text-base font-semibold">A-01-02-03</div>
          <span className="text-[11px] px-1.5 py-0.5 bg-muted rounded text-muted-foreground">
            常温区 22.3℃
          </span>
        </div>

        <div className="text-xs text-muted-foreground mb-2">扫货核对</div>
        <ScanInput
          mode="scanner"
          placeholder="扫追溯码（已扫 478/480）"
          onScan={(code) => setScanned(code)}
          lastScanned={scanned}
        />
      </div>

      {/* 字段 */}
      <div className="bg-background px-3 py-3 border-b">
        <FieldTable size="sm" rows={FIELDS} />
      </div>

      {/* 盈亏对比 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-sm font-semibold mb-2 text-wms-warning">
            盘点差异
          </div>
          <DiffPanel
            layout="stacked"
            highlightChanged
            before={{
              "账面数量": "480 瓶",
              "可用": "468 瓶",
              "状态": "正常",
            }}
            after={{
              "实盘数量": "478 瓶（-2）",
              "可用": "466 瓶",
              "状态": "待审核（盘亏 0.42%）",
            }}
          />
          <div className="mt-3 text-xs text-muted-foreground">
            盘亏率 0.42% &lt; 1%（无需主管审核），可直接提交调账
          </div>
        </Card>

        {/* 备注 */}
        <div className="mt-3">
          <div className="text-sm font-semibold mb-1.5">差异原因</div>
          <textarea
            placeholder="说明盘亏原因……"
            rows={2}
            className="w-full px-3 py-2 text-sm rounded-md border bg-background resize-none"
            defaultValue="第一盘 478，复盘 478，确认实盘 478。疑似前期出库少记 2 瓶"
          />
        </div>

        {/* 双盘信息 */}
        <Card className="p-3 mt-3">
          <div className="text-sm font-semibold mb-2">双盘记录</div>
          <div className="text-xs flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">第一盘 · 张三 (u001)</span>
              <span className="font-medium">478 瓶 · 10:15:42</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">第二盘 · 李四 (u002)</span>
              <span className="font-medium text-primary">478 瓶 · 10:32:18 ✓</span>
            </div>
            <div className="flex items-center justify-between pt-1.5 border-t">
              <span className="text-muted-foreground">两盘是否一致</span>
              <span className="font-medium text-wms-success">是</span>
            </div>
          </div>
        </Card>
      </div>

      {/* 底部 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">回退</Button>
        <Button className="flex-[2] h-10">提交本位 + 下一位</Button>
      </div>
    </div>
  );
}
