import { useState } from "react";
import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import {
  PageHeader,
  StatusBadge,
  TempChart,
  type TempPoint,
} from "@wms/ui";
import { MapPin, Truck, AlertTriangle, Snowflake, Phone } from "lucide-react";

/**
 * M10InTransitTemp — M10-002 在途温控
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M10-002（接收 TMS 温控数据 / 实时曲线 / 偏差登记）
 * Wave：Wave 4.0（M10 运输协同）
 * 业务约束：接收外部 TMS 数据；偏差需登记 + 通知货主；冷链全程温度记录留 5 年
 *
 * @example
 *   <M10InTransitTemp />
 */

interface Shipment {
  id: string;
  truck: string;
  driver: string;
  route: string;
  startAt: string;
  eta: string;
  itemCount: number;
  current: number;
  min: number;
  max: number;
  status: "normal" | "warning" | "excursion";
  excursionCount: number;
}

const SHIPMENTS: Shipment[] = [
  { id: "TR-2026-0521", truck: "京 A·12345", driver: "李司机", route: "天竺仓 → 同仁堂朝阳店",
    startAt: "今日 08:30", eta: "今日 11:00", itemCount: 24, current: 5.6, min: 2, max: 8, status: "normal", excursionCount: 0 },
  { id: "TR-2026-0522", truck: "京 B·67890", driver: "王司机", route: "天竺仓 → 国大药房海淀店",
    startAt: "今日 09:15", eta: "今日 12:45", itemCount: 18, current: 8.7, min: 2, max: 8, status: "warning", excursionCount: 1 },
  { id: "TR-2026-0518", truck: "京 C·11122", driver: "张司机", route: "天竺仓 → 益丰大药房通州店",
    startAt: "今日 07:00", eta: "今日 09:30 (已到)", itemCount: 12, current: 6.2, min: 2, max: 8, status: "normal", excursionCount: 0 },
];

function genTemp(base: number, amp: number, excursionAt?: number): TempPoint[] {
  const points: TempPoint[] = [];
  for (let i = 0; i < 40; i++) {
    const min_offset = i * 5;
    const hour = 8 + Math.floor(min_offset / 60);
    const min = min_offset % 60;
    const t = `${String(hour).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
    let v = base + Math.sin(i / 6) * amp + (Math.random() - 0.5) * 0.3;
    if (excursionAt !== undefined && i >= excursionAt && i < excursionAt + 3) {
      v = base + amp * 3;
    }
    points.push({ t, v: Math.round(v * 10) / 10 });
  }
  return points;
}

const TEMP_DATA: Record<string, TempPoint[]> = {
  "TR-2026-0521": genTemp(5.5, 0.8),
  "TR-2026-0522": genTemp(6, 1.0, 22), // 22 号点温度偏高
  "TR-2026-0518": genTemp(5.8, 0.6),
};

export function M10InTransitTemp() {
  const [selected, setSelected] = useState<string>("TR-2026-0522");
  const ship = SHIPMENTS.find((s) => s.id === selected) ?? SHIPMENTS[0];
  const points = TEMP_DATA[selected];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="在途温控"
        subtitle="M10-002 · 接收 TMS 实时温度 · 含 GSP 全程台账"
        actions={
          <>
            <Button variant="outline" size="sm">
              <Phone data-icon="inline-start" /> 联系司机
            </Button>
            <Button variant="outline" size="sm">导出温度记录</Button>
          </>
        }
      />

      {/* 在途车辆卡片 */}
      <div className="px-6 py-4 border-b">
        <div className="flex items-center justify-between mb-3">
          <div className="text-sm font-semibold">在途车辆（3 辆）</div>
          <span className="text-xs text-muted-foreground">数据源：京东 TMS · 5 分钟刷新</span>
        </div>
        <div className="grid grid-cols-3 gap-3">
          {SHIPMENTS.map((s) => {
            const isActive = s.id === selected;
            const colorClass = s.status === "warning" ? "border-wms-warning/50" : "";
            return (
              <Card
                key={s.id}
                className={`p-3 cursor-pointer ${colorClass} ${isActive ? "ring-2 ring-primary" : ""}`}
                onClick={() => setSelected(s.id)}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="font-mono text-sm font-semibold">{s.id}</div>
                    <div className="text-xs text-muted-foreground mt-0.5">{s.truck} · {s.driver}</div>
                  </div>
                  {s.status === "warning" ? (
                    <StatusBadge status="pending" size="sm" label={`偏差 ${s.excursionCount}`} />
                  ) : (
                    <StatusBadge status="qualified" size="sm" label="正常" />
                  )}
                </div>
                <div className="flex items-baseline gap-2 mt-2">
                  <Snowflake className="size-4 text-wms-cold flex-shrink-0" />
                  <span className={`text-2xl font-bold ${
                    s.status === "warning" ? "text-wms-warning" : "text-wms-cold"
                  }`}>{s.current}</span>
                  <span className="text-xs text-muted-foreground">℃</span>
                  <span className="ml-auto text-[11px] text-muted-foreground">阈值 {s.min}-{s.max}℃</span>
                </div>
                <div className="text-xs text-muted-foreground mt-2 flex items-center gap-1">
                  <MapPin className="size-3" />
                  <span className="truncate">{s.route}</span>
                </div>
                <div className="text-[11px] text-muted-foreground mt-1 flex items-center gap-2">
                  <Truck className="size-3" />
                  <span>出发 {s.startAt}</span>
                  <span>ETA {s.eta}</span>
                </div>
              </Card>
            );
          })}
        </div>
      </div>

      {/* 温度曲线 */}
      <div className="px-6 py-4 border-b">
        <div className="flex items-center justify-between mb-3">
          <div>
            <div className="text-sm font-semibold">{ship.id} · 在途温度曲线（{ship.driver} · {ship.truck}）</div>
            <div className="text-xs text-muted-foreground mt-0.5">
              出发以来 {points.length * 5} 分钟 · 阈值 {ship.min}-{ship.max}℃
            </div>
          </div>
          {ship.status === "warning" && (
            <div className="flex items-center gap-2">
              <span className="text-xs text-wms-warning flex items-center gap-1">
                <AlertTriangle data-icon="inline-start" /> 1 次偏差，需登记
              </span>
              <Button variant="outline" size="sm">登记偏差</Button>
            </div>
          )}
        </div>
        <TempChart
          points={points}
          minThreshold={ship.min}
          maxThreshold={ship.max}
          height={240}
        />
      </div>

      {/* GSP 台账 */}
      <div className="px-6 py-4 grid grid-cols-2 gap-4">
        <Card className="p-4 bg-muted/30">
          <div className="text-sm font-semibold mb-2">GSP 第 64 条 · 全程台账</div>
          <ul className="text-xs text-muted-foreground flex flex-col gap-1">
            <li>· 全程温度记录（每 5 分钟一点）</li>
            <li>· 出发-到货完整时段</li>
            <li>· 偏差需登记 + 通知货主</li>
            <li>· 留存 5 年（电子+纸质）</li>
            <li>· 数据由 TMS 推送 + WMS 接收 + ERP 报送药监</li>
          </ul>
        </Card>

        <Card className="p-4 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-sm font-semibold mb-2 text-wms-warning">本月偏差汇总</div>
          <div className="text-xs flex flex-col gap-1.5">
            <div className="flex justify-between">
              <span className="text-muted-foreground">在途偏差次数</span>
              <span className="font-semibold">3 次</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">已登记原因</span>
              <span className="font-semibold text-wms-success">3 / 3</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">已通知货主</span>
              <span className="font-semibold text-wms-success">3 / 3</span>
            </div>
            <div className="flex justify-between border-t pt-1.5 mt-1.5">
              <span className="text-muted-foreground">全程合规率</span>
              <span className="font-semibold">99.91%</span>
            </div>
          </div>
        </Card>
      </div>
    </div>
  );
}
