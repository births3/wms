import { useState } from "react";
import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import {
  PageHeader,
  StatusBadge,
  TempChart,
  type TempPoint,
} from "@wms/ui";
import { Snowflake, AlertTriangle, Download, Bell } from "lucide-react";

/**
 * M5ColdMonitor — M5-002 冷链监控
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M5-002（冷链温度实时监控 / 阈值告警 / 历史曲线 / 偏差登记）
 * Wave：Wave 3.0（M5 冷链）
 * 业务约束：CR 区 2-8℃（>15min 偏差告警）；FR 区 ≤-15℃；超标自动锁定库位
 *
 * @example
 *   <M5ColdMonitor />
 */

interface ZoneInfo {
  code: string;
  name: string;
  type: "CR" | "FR" | "CL";
  min: number;
  max: number;
  current: number;
  status: "normal" | "excursion" | "warning";
}

const ZONES: ZoneInfo[] = [
  { code: "C", name: "C 冷藏区", type: "CR", min: 2, max: 8, current: 5.2, status: "normal" },
  { code: "D", name: "D 冷冻区", type: "FR", min: -30, max: -15, current: -26.8, status: "normal" },
  { code: "B", name: "B 阴凉区", type: "CL", min: 0, max: 20, current: 18.5, status: "warning" },
];

// 24 小时温度数据（每 5 分钟一个点 → 288 点；这里取 60 个代表点）
function genTemp(base: number, amp: number, excursionAt?: number): TempPoint[] {
  const points: TempPoint[] = [];
  for (let i = 0; i < 60; i++) {
    const hour = Math.floor(i * 24 / 60);
    const min = Math.floor((i * 24 / 60 - hour) * 60);
    const t = `${String(hour).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
    let v = base + Math.sin(i / 8) * amp + (Math.random() - 0.5) * 0.4;
    if (excursionAt !== undefined && i >= excursionAt && i < excursionAt + 4) {
      v = base + amp * 2.5; // 偏差点
    }
    points.push({ t, v: Math.round(v * 10) / 10 });
  }
  return points;
}

const POINTS_C = genTemp(5, 1.5);
const POINTS_D = genTemp(-26, 1.0);
const POINTS_B = genTemp(15, 3, 38); // 38 号点附近偏差到 21℃

const ALERTS = [
  { time: "今 14:23", zone: "B 阴凉区", severity: "warning", desc: "温度 21.2℃ 持续 18 分钟（阈值 ≤20℃）" },
  { time: "今 09:08", zone: "C 冷藏区", severity: "info", desc: "短暂尖峰 9.1℃（持续 3 分钟，未告警）" },
  { time: "昨 22:45", zone: "D 冷冻区", severity: "warning", desc: "温度 -14.6℃ 持续 8 分钟（阈值 ≤-15℃）" },
];

export function M5ColdMonitor() {
  const [selected, setSelected] = useState<string>("B");
  const zone = ZONES.find((z) => z.code === selected) ?? ZONES[0];
  const points = selected === "C" ? POINTS_C : selected === "D" ? POINTS_D : POINTS_B;

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="冷链温度监控"
        subtitle="M5-002 · 实时数据采集 · 24h 历史曲线 · GSP 第 64 条"
        actions={
          <>
            <Button variant="outline" size="sm">
              <Bell data-icon="inline-start" /> 告警配置
            </Button>
            <Button variant="outline" size="sm">
              <Download data-icon="inline-start" /> 导出温度记录
            </Button>
          </>
        }
      />

      {/* 温区选择卡片 */}
      <div className="px-6 py-4 border-b grid grid-cols-3 gap-3">
        {ZONES.map((z) => {
          const isActive = z.code === selected;
          const colorClass =
            z.status === "excursion" ? "border-destructive bg-destructive/5" :
            z.status === "warning" ? "border-wms-warning bg-wms-warning/5" :
            "border-wms-cold/30";
          return (
            <Card
              key={z.code}
              className={`p-4 cursor-pointer ${colorClass} ${isActive ? "ring-2 ring-primary ring-offset-2" : ""}`}
              onClick={() => setSelected(z.code)}
            >
              <div className="flex items-start justify-between mb-2">
                <div>
                  <div className="flex items-center gap-1.5">
                    <Snowflake className="size-4 text-wms-cold" />
                    <span className="font-semibold text-base">{z.name}</span>
                    <span className="text-[11px] px-1.5 py-0.5 bg-background border rounded font-medium">{z.type}</span>
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    设定 {z.min}℃ ~ {z.max}℃
                  </div>
                </div>
                {z.status === "warning" ? (
                  <StatusBadge status="pending" size="sm" label="超标" />
                ) : (
                  <StatusBadge status="qualified" size="sm" label="正常" />
                )}
              </div>
              <div className="flex items-baseline gap-2 mt-3">
                <span className={`text-3xl font-bold ${
                  z.status === "warning" ? "text-wms-warning" : "text-wms-cold"
                }`}>{z.current}</span>
                <span className="text-sm text-muted-foreground">℃</span>
                <span className="ml-auto text-xs text-muted-foreground">实时 5s 前</span>
              </div>
            </Card>
          );
        })}
      </div>

      {/* 温度曲线 */}
      <div className="px-6 py-4 border-b">
        <div className="flex items-center justify-between mb-3">
          <div>
            <div className="text-sm font-semibold">{zone.name} · 24 小时温度曲线</div>
            <div className="text-xs text-muted-foreground mt-0.5">
              共 288 个采样点（每 5 分钟）· 阈值红色虚线
            </div>
          </div>
          <div className="flex gap-1">
            <Button variant="outline" size="sm">实时</Button>
            <Button variant="outline" size="sm">24h</Button>
            <Button variant="outline" size="sm">7d</Button>
            <Button variant="outline" size="sm">30d</Button>
          </div>
        </div>
        <TempChart
          points={points}
          minThreshold={zone.min}
          maxThreshold={zone.max}
          height={280}
        />
      </div>

      {/* 告警列表 */}
      <div className="px-6 py-4">
        <div className="flex items-center justify-between mb-3">
          <div className="text-sm font-semibold">最近 24 小时温度事件</div>
          <span className="text-xs text-muted-foreground">3 条 · 全部已确认</span>
        </div>
        <div className="flex flex-col gap-2">
          {ALERTS.map((alert, i) => (
            <Card key={i} className={`p-3 ${
              alert.severity === "warning" ? "border-wms-warning/30 bg-wms-warning/5" : ""
            }`}>
              <div className="flex items-start gap-3">
                <AlertTriangle className={`size-4 flex-shrink-0 mt-0.5 ${
                  alert.severity === "warning" ? "text-wms-warning" : "text-muted-foreground"
                }`} />
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-0.5">
                    <span className="text-sm font-medium">{alert.zone}</span>
                    {alert.severity === "warning" ? (
                      <span className="text-[10px] px-1.5 py-0.5 bg-wms-warning/10 text-wms-warning rounded font-medium">告警</span>
                    ) : (
                      <span className="text-[10px] px-1.5 py-0.5 bg-muted rounded text-muted-foreground">提示</span>
                    )}
                    <span className="text-xs text-muted-foreground ml-auto">{alert.time}</span>
                  </div>
                  <div className="text-xs text-muted-foreground">{alert.desc}</div>
                </div>
                <Button variant="outline" size="sm" className="h-7 text-xs">详情</Button>
              </div>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
