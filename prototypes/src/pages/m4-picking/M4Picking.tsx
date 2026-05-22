import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useState } from "react";
import { ChevronLeft, MapPin, Package, AlertTriangle } from "lucide-react";
import {
  StatusBadge,
  OfflineIndicator,
  ScanInput,
  StepFlow,
} from "@/components/business";

/**
 * M4Picking — M4-003 PDA 拣货
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M4-003（PDA 按 SO 拣货 / FIFO 推荐 / 强制扫货+扫位 / 一致性校验）
 * Wave：Wave 2.5（M4 出库 PDA 业务流）
 * 业务约束：必须按 FIFO 推荐顺序；扫错位要红色提示；冷链商品分批拣
 *
 * @example
 *   <M4Picking />
 */

interface PickItem {
  line: number;
  itemCode: string;
  itemName: string;
  spec: string;
  /** FIFO 推荐：按效期最近优先 */
  batch: string;
  expiry: string;
  location: string;
  qty: number;
  picked: number;
  status: "pending" | "in_progress" | "completed";
  isCold: boolean;
}

const PICK_LIST: PickItem[] = [
  { line: 1, itemCode: "P-001234", itemName: "葡萄糖注射液", spec: "500ml × 24",
    batch: "20250901A", expiry: "2027-09-01", location: "A-01-02-03", qty: 5, picked: 5, status: "completed", isCold: false },
  { line: 2, itemCode: "P-001234", itemName: "葡萄糖注射液", spec: "500ml × 24",
    batch: "20260301A", expiry: "2028-03-01", location: "A-01-02-04", qty: 3, picked: 0, status: "in_progress", isCold: false },
  { line: 3, itemCode: "P-001235", itemName: "重组人胰岛素", spec: "3ml:300IU × 5",
    batch: "20260315B", expiry: "2027-03-15", location: "C-01-01-08", qty: 2, picked: 0, status: "pending", isCold: true },
  { line: 4, itemCode: "P-002001", itemName: "盐酸吗啡片", spec: "10mg × 100",
    batch: "20260101N", expiry: "2027-01-01", location: "Q-01-01-01", qty: 1, picked: 0, status: "pending", isCold: false },
];

export function M4Picking() {
  const [scanned, setScanned] = useState<string>("");
  const total = PICK_LIST.length;
  const completed = PICK_LIST.filter((i) => i.status === "completed").length;
  const currentItem = PICK_LIST.find((i) => i.status === "in_progress");

  return (
    <div data-device="pda" className="w-[480px] min-h-[900px] flex flex-col rounded-xl border overflow-hidden shadow-md font-sans bg-muted/30">
      <OfflineIndicator state="online" />

      {/* 顶栏 */}
      <div className="bg-background px-3 py-2 border-b flex items-center gap-2">
        <Button variant="ghost" size="icon" className="h-8 w-8">
          <ChevronLeft className="h-5 w-5" />
        </Button>
        <div className="flex-1">
          <div className="text-xs text-muted-foreground">M4-003 PDA 拣货 · 单 SO-2026-0042</div>
          <div className="text-sm font-semibold">已拣 {completed}/{total} 项</div>
        </div>
        <StatusBadge status="in_progress" size="sm" label="拣货中" />
      </div>

      {/* 进度 */}
      <div className="bg-background px-3 py-3 border-b">
        <StepFlow
          current={1}
          size="sm"
          steps={[
            { label: "扫单", description: "SO-2026-0042 ✓" },
            { label: "FIFO 拣货", description: `${completed}/${total}` },
            { label: "复核交接" },
          ]}
        />
      </div>

      {/* 当前拣货项 */}
      {currentItem && (
        <div className="bg-background px-3 py-3 border-b">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs text-muted-foreground">当前拣货 · 第 {currentItem.line} 行</span>
            <span className="text-[11px] px-1.5 py-0.5 bg-primary/10 text-primary rounded font-medium">FIFO 推荐</span>
          </div>
          <Card className="p-3 border-primary/40 bg-primary/5">
            <div className="flex items-start justify-between mb-2">
              <div>
                <div className="text-sm font-semibold">{currentItem.itemName}</div>
                <div className="text-xs text-muted-foreground mt-0.5">{currentItem.spec}</div>
              </div>
              <div className="text-right">
                <div className="text-2xl font-bold text-primary">{currentItem.qty}</div>
                <div className="text-[11px] text-muted-foreground">瓶</div>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs mt-2 pt-2 border-t border-primary/20">
              <div>
                <div className="text-muted-foreground">批号 / 效期</div>
                <div className="font-mono mt-0.5">{currentItem.batch}</div>
                <div className="text-[11px] text-muted-foreground">效期 {currentItem.expiry}</div>
              </div>
              <div>
                <div className="text-muted-foreground flex items-center gap-1">
                  <MapPin className="h-3 w-3" /> 库位
                </div>
                <div className="font-mono mt-0.5 text-primary font-semibold">{currentItem.location}</div>
              </div>
            </div>
          </Card>

          <div className="mt-3">
            <ScanInput
              mode="scanner"
              placeholder="扫库位 → 扫货 → 提交"
              onScan={(code) => setScanned(code)}
              lastScanned={scanned}
            />
          </div>
        </div>
      )}

      {/* 拣货清单 */}
      <div className="flex-1 overflow-y-auto bg-muted/30 px-3 py-3">
        <div className="text-xs text-muted-foreground mb-2 flex items-center justify-between">
          <span>拣货清单（按 FIFO 排序）</span>
          <span className="font-mono">SO-2026-0042 · 11 件</span>
        </div>
        <div className="space-y-1.5">
          {PICK_LIST.map((item) => (
            <Card
              key={item.line}
              className={`p-2.5 ${
                item.status === "completed" ? "opacity-60" :
                item.status === "in_progress" ? "border-primary border-2" :
                ""
              }`}
            >
              <div className="flex items-center gap-2">
                <span className="text-[11px] font-mono text-muted-foreground w-6">#{item.line}</span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="text-sm font-medium truncate">{item.itemName}</span>
                    {item.isCold && (
                      <span className="text-[10px] px-1 py-0.5 bg-wms-cold/10 text-wms-cold rounded">❄️</span>
                    )}
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    <span className="font-mono">{item.location}</span> · 批 {item.batch}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-semibold">
                    {item.picked}/{item.qty}
                  </div>
                  {item.status === "completed" && (
                    <span className="text-[10px] text-wms-success">✓</span>
                  )}
                </div>
              </div>
            </Card>
          ))}
        </div>
      </div>

      {/* 底部 */}
      <div className="bg-background px-3 py-2 border-t flex gap-2">
        <Button variant="outline" className="flex-1 h-10">
          <AlertTriangle className="h-4 w-4 mr-1" /> 异常
        </Button>
        <Button variant="outline" className="flex-1 h-10">
          <Package className="h-4 w-4 mr-1" /> 短拣
        </Button>
        <Button className="flex-[2] h-10">提交本行</Button>
      </div>
    </div>
  );
}
