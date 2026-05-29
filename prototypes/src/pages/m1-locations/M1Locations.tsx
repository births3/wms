import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import {
  PageHeader,
  StatusBadge,
} from "@wms/ui";
import { Plus, Thermometer, MapPin, Package } from "lucide-react";

interface Zone {
  code: string;
  name: string;
  type: "RT" | "CR" | "CL" | "FR";
  tempRange: string;
  /** 库位数 */
  total: number;
  /** 已用库位 */
  used: number;
  /** 当前温度 */
  currentTemp: string;
  /** 是否超标 */
  excursion: boolean;
}

const ZONES: Zone[] = [
  { code: "A", name: "A 常温区", type: "RT", tempRange: "10-30℃", total: 480, used: 286, currentTemp: "22.3℃", excursion: false },
  { code: "B", name: "B 阴凉区", type: "CL", tempRange: "≤20℃", total: 240, used: 180, currentTemp: "18.5℃", excursion: false },
  { code: "C", name: "C 冷藏区", type: "CR", tempRange: "2-8℃", total: 120, used: 95, currentTemp: "5.2℃", excursion: false },
  { code: "D", name: "D 冷冻区", type: "FR", tempRange: "≤-25℃", total: 60, used: 38, currentTemp: "-26.8℃", excursion: false },
  { code: "Q", name: "Q 隔离区", type: "RT", tempRange: "10-30℃", total: 24, used: 8, currentTemp: "23.1℃", excursion: false },
  { code: "R", name: "R 不合格区", type: "RT", tempRange: "10-30℃", total: 12, used: 3, currentTemp: "22.8℃", excursion: false },
];

const ZONE_COLOR = {
  RT: { bg: "bg-muted/50", border: "border-muted-foreground/20", text: "text-muted-foreground" },
  CR: { bg: "bg-wms-cold/10", border: "border-wms-cold/30", text: "text-wms-cold" },
  CL: { bg: "bg-wms-cold/5", border: "border-wms-cold/20", text: "text-wms-cold/70" },
  FR: { bg: "bg-wms-cold/20", border: "border-wms-cold/40", text: "text-wms-cold font-semibold" },
};

/**
 * M1Locations — M1-004 仓库与库位管理
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M1-004（仓库 / 温区 / 库位 / 占用率 / 实时温度）
 * Wave：Wave 2.0（M1 基础数据）
 * 业务约束：温区必须设温度阈值；冷链区必须接温度采集；隔离区/不合格区独立
 *
 * @example
 *   <M1Locations />
 */
export function M1Locations() {
  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="仓库与库位管理"
        subtitle="W001 北京天竺仓 · 6 个温区 · 936 个库位 · GSP 温区规范"
        actions={
          <>
            <Button variant="outline" size="sm">温区配置</Button>
            <Button variant="outline" size="sm">库位导入</Button>
            <Button size="sm">
              <Plus data-icon="inline-start" /> 新增库位
            </Button>
          </>
        }
      />

      {/* 温区卡片网格 */}
      <div className="px-6 py-4 border-b">
        <div className="text-sm font-semibold mb-3">温区分布</div>
        <div className="grid grid-cols-3 gap-3">
          {ZONES.map((z) => {
            const c = ZONE_COLOR[z.type];
            const occupancyPct = Math.round((z.used / z.total) * 100);
            return (
              <Card key={z.code} className={`p-4 border-2 ${c.border} ${c.bg}`}>
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`text-lg font-bold ${c.text}`}>{z.name}</span>
                      <span className={`text-[11px] px-1.5 py-0.5 rounded font-medium ${c.text} bg-background border ${c.border}`}>
                        {z.type}
                      </span>
                    </div>
                    <div className="text-xs text-muted-foreground flex items-center gap-1">
                      <Thermometer className="size-3" />
                      <span>设定 {z.tempRange}</span>
                    </div>
                  </div>
                  {z.excursion ? (
                    <StatusBadge status="unqualified" size="sm" label="超标" />
                  ) : (
                    <StatusBadge status="qualified" size="sm" label="正常" />
                  )}
                </div>

                <div className="mt-3">
                  <div className="flex items-baseline justify-between mb-1">
                    <span className="text-xs text-muted-foreground">实时温度</span>
                    <span className={`text-base font-bold ${c.text}`}>{z.currentTemp}</span>
                  </div>
                </div>

                <div className="mt-3">
                  <div className="flex items-center justify-between text-xs mb-1">
                    <span className="text-muted-foreground">库位占用</span>
                    <span className="font-medium">{z.used} / {z.total} ({occupancyPct}%)</span>
                  </div>
                  <div className="h-2 bg-background rounded-full overflow-hidden">
                    <div
                      className={`h-full ${
                        occupancyPct >= 90 ? "bg-destructive" :
                        occupancyPct >= 70 ? "bg-wms-warning" :
                        "bg-primary"
                      }`}
                      style={{ width: `${occupancyPct}%` }}
                    />
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      </div>

      {/* 库位详情：A 区平面图（示意） */}
      <div className="px-6 py-4">
        <div className="flex items-center justify-between mb-3">
          <div className="text-sm font-semibold">A 常温区 — 平面图（示意）</div>
          <div className="flex items-center gap-3 text-xs">
            <div className="flex items-center gap-1">
              <span className="size-3 rounded bg-muted/50 border" />
              <span>空闲</span>
            </div>
            <div className="flex items-center gap-1">
              <span className="size-3 rounded bg-primary/30 border" />
              <span>有货</span>
            </div>
            <div className="flex items-center gap-1">
              <span className="size-3 rounded bg-destructive/30 border" />
              <span>满</span>
            </div>
          </div>
        </div>
        <Card className="p-4 bg-muted/30">
          <div className="grid grid-cols-12 gap-1">
            {Array.from({ length: 96 }).map((_, i) => {
              const occupancy = (i * 17 + 23) % 100;
              const cls = occupancy >= 90 ? "bg-destructive/30 border-destructive/50" :
                          occupancy >= 50 ? "bg-primary/30 border-primary/50" :
                          occupancy > 0 ? "bg-primary/10 border-primary/20" :
                          "bg-background border-input";
              return (
                <div
                  key={i}
                  className={`aspect-square border rounded text-[9px] flex items-center justify-center font-mono ${cls}`}
                  title={`A-01-${String(Math.floor(i / 12) + 1).padStart(2, "0")}-${String((i % 12) + 1).padStart(2, "0")}`}
                >
                  {String(Math.floor(i / 12) + 1).padStart(2, "0")}-{String((i % 12) + 1).padStart(2, "0")}
                </div>
              );
            })}
          </div>
          <div className="text-xs text-muted-foreground mt-3 flex items-center gap-3">
            <MapPin className="size-3" />
            <span>共 96 个库位（8 排 × 12 列）· hover 显示编码 / 点击查看库存</span>
            <span className="ml-auto flex items-center gap-1">
              <Package data-icon="inline-start" /> 当前占用 53 / 96
            </span>
          </div>
        </Card>
      </div>
    </div>
  );
}
