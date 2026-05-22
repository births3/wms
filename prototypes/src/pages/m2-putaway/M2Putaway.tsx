import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { ChevronLeft, MapPin, Thermometer } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  ScanInput,
  StepFlow,
} from "@/components/business";

/**
 * M2Putaway — M2-005 PDA 上架
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-005（PDA 上架 / 系统推荐库位 / 强制扫货+扫位 / 温区匹配）
 * Wave：Wave 1.5（M2 PDA 业务流）
 * 业务约束：必须扫货且扫位；冷链商品必须放冷库；推荐库位优先（FIFO + 温区匹配）
 *
 * @example
 *   <M2Putaway />
 */

interface SuggestedLocation {
  code: string;
  area: string;
  zone: string;
  occupancy: string;
  recommended?: boolean;
}

const SUGGESTED: SuggestedLocation[] = [
  { code: "A-01-02-03", area: "常温区", zone: "RT", occupancy: "20%", recommended: true },
  { code: "A-01-02-04", area: "常温区", zone: "RT", occupancy: "45%" },
  { code: "A-02-01-01", area: "常温区", zone: "RT", occupancy: "10%" },
];

export function M2Putaway() {
  const [scannedItem, setScannedItem] = useState<string>("AB12-CD34-EF56");
  const [scannedLoc, setScannedLoc] = useState<string>("");
  const [selectedLoc, setSelectedLoc] = useState<string>("A-01-02-03");

  const isItemScanned = !!scannedItem;
  const isLocScanned = !!scannedLoc;

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="h-8 w-8">
          <ChevronLeft className="h-5 w-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M2-005 PDA 上架</div>
          <div className="text-sm font-semibold">待上架 · 12 件</div>
        </div>
        <StatusBadge status="in_progress" size="sm" label="进行中" />
      </div>

      {/* 进度 */}
      <div className="bg-background px-3 py-3 border-b">
        <StepFlow
          current={1}
          size="sm"
          steps={[
            { label: "扫货", description: isItemScanned ? "✓" : "" },
            { label: "扫库位", description: isLocScanned ? "✓" : "等待" },
            { label: "确认入库" },
          ]}
        />
      </div>

      {/* 商品信息 */}
      <div className="bg-background px-3 py-3 border-b">
        <div className="text-xs text-muted-foreground mb-1">① 扫货</div>
        <Card className="p-3 mb-2">
          <div className="flex items-start justify-between mb-1">
            <div>
              <div className="text-sm font-semibold">葡萄糖注射液 500ml</div>
              <div className="text-xs text-muted-foreground mt-0.5">批号 20260301A · 24 瓶</div>
            </div>
            <span className="text-[11px] px-1.5 py-0.5 bg-muted rounded font-medium">RT 常温</span>
          </div>
          <div className="font-mono text-xs text-primary">{scannedItem}</div>
        </Card>
        <ScanInput
          mode="scanner"
          placeholder="扫追溯码或托盘"
          onScan={(code) => setScannedItem(code)}
          lastScanned={scannedItem}
        />
      </div>

      {/* 推荐库位 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <div className="text-xs text-muted-foreground mb-2">② 选择库位</div>

        <div className="space-y-2 mb-3">
          {SUGGESTED.map((loc) => (
            <Card
              key={loc.code}
              className={`p-3 ${
                selectedLoc === loc.code ? "border-primary border-2 bg-primary/5" : ""
              }`}
              onClick={() => setSelectedLoc(loc.code)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <MapPin className="h-4 w-4 text-primary" />
                  <span className="font-mono text-sm font-semibold">{loc.code}</span>
                  {loc.recommended && (
                    <span className="text-[11px] px-1.5 py-0.5 bg-wms-success/10 text-wms-success rounded font-medium">
                      推荐
                    </span>
                  )}
                </div>
                <div className="text-xs text-muted-foreground">{loc.occupancy}</div>
              </div>
              <div className="flex items-center gap-3 text-xs text-muted-foreground mt-1.5">
                <span>{loc.area}</span>
                <span className="flex items-center gap-1">
                  <Thermometer className="h-3 w-3" />
                  {loc.zone}
                </span>
              </div>
            </Card>
          ))}
        </div>

        <ScanInput
          mode="scanner"
          placeholder="扫库位条码"
          onScan={(code) => setScannedLoc(code)}
          lastScanned={scannedLoc}
        />
      </div>

      {/* 底部操作栏 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">放弃</Button>
        <Button
          className="flex-[2] h-10"
          disabled={!isItemScanned || !isLocScanned}
        >
          确认上架到 {selectedLoc}
        </Button>
      </div>
    </div>
  );
}
