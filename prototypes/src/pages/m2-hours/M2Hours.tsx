import { Card } from "@wms/ui";
import { Button } from "@wms/ui";
import { Input } from "@wms/ui";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@wms/ui";
import {
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@wms/ui";
import { Download, TrendingUp, Users, Clock, Package } from "lucide-react";

interface WorkRow {
  date: string;
  worker: string;
  asnCount: number;
  itemQty: number;
  /** 工时分钟 */
  minutes: number;
  /** 平均件/小时 */
  efficiency: number;
  exception: number;
}

const MOCK: WorkRow[] = [
  { date: "2026-04-30", worker: "张三 (u001)", asnCount: 12, itemQty: 1840, minutes: 386, efficiency: 286, exception: 0 },
  { date: "2026-04-30", worker: "李四 (u002)", asnCount: 9, itemQty: 1320, minutes: 358, efficiency: 221, exception: 1 },
  { date: "2026-04-30", worker: "王五 (u003)", asnCount: 8, itemQty: 980, minutes: 312, efficiency: 188, exception: 0 },
  { date: "2026-04-29", worker: "张三 (u001)", asnCount: 14, itemQty: 2120, minutes: 412, efficiency: 309, exception: 1 },
  { date: "2026-04-29", worker: "李四 (u002)", asnCount: 7, itemQty: 1080, minutes: 278, efficiency: 233, exception: 0 },
  { date: "2026-04-29", worker: "赵六 (u004)", asnCount: 5, itemQty: 580, minutes: 215, efficiency: 162, exception: 2 },
  { date: "2026-04-28", worker: "张三 (u001)", asnCount: 10, itemQty: 1560, minutes: 342, efficiency: 274, exception: 0 },
  { date: "2026-04-28", worker: "王五 (u003)", asnCount: 11, itemQty: 1620, minutes: 388, efficiency: 250, exception: 1 },
];

/**
 * M2Hours — M2-009 收货工时统计
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-009（PC 收货工时月度统计 / 多端工时合并 / 效率排行）
 * Wave：Wave 2.5（M2 报表）
 * 业务约束：工时 = PDA 验收实际时长（首扫到提交）；多端协作工时合并到主操作人
 *
 * @example
 *   <M2Hours />
 */
export function M2Hours() {
  const cols: DataTableColumn<WorkRow>[] = [
    { key: "date", header: "日期",
      render: (r) => <span className="font-mono text-xs">{r.date}</span> },
    { key: "worker", header: "收货员",
      render: (r) => <span className="text-sm">{r.worker}</span> },
    { key: "asn", header: "ASN 数", align: "right",
      render: (r) => <span className="text-sm">{r.asnCount}</span> },
    { key: "qty", header: "件数", align: "right",
      render: (r) => <span className="text-sm">{r.itemQty}</span> },
    { key: "minutes", header: "工时", align: "right",
      render: (r) => <span className="text-sm">{Math.floor(r.minutes / 60)}h {r.minutes % 60}min</span> },
    { key: "eff", header: "件/小时", align: "right",
      render: (r) => {
        const color = r.efficiency >= 250 ? "text-wms-success" :
                      r.efficiency >= 200 ? "" : "text-wms-warning";
        return <span className={`text-sm font-semibold ${color}`}>{r.efficiency}</span>;
      } },
    { key: "exception", header: "异常", align: "right",
      render: (r) => r.exception > 0 ? (
        <span className="text-xs text-destructive">{r.exception}</span>
      ) : <span className="text-xs text-muted-foreground">—</span> },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="收货工时统计"
        subtitle="M2-009 · 月度工时分析 + 效率排行 + 异常追溯"
        actions={
          <Button variant="outline" size="sm">
            <Download data-icon="inline-start" /> 导出 Excel
          </Button>
        }
      />

      {/* 筛选 */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3 items-end">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">统计周期</label>
          <Select defaultValue="month">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="day">日</SelectItem>
              <SelectItem value="week">周</SelectItem>
              <SelectItem value="month">月</SelectItem>
              <SelectItem value="quarter">季度</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">起始日期</label>
          <Input type="date" defaultValue="2026-04-01" />
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">截止日期</label>
          <Input type="date" defaultValue="2026-04-30" />
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">收货员</label>
          <Select defaultValue="all">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="u001">张三 (u001)</SelectItem>
              <SelectItem value="u002">李四 (u002)</SelectItem>
              <SelectItem value="u003">王五 (u003)</SelectItem>
              <SelectItem value="u004">赵六 (u004)</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      {/* KPI */}
      <div className="px-6 py-4 border-b grid grid-cols-4 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground flex items-center gap-1">
            <Package data-icon="inline-start" /> 总收货件数
          </div>
          <div className="text-2xl font-bold mt-1">28,640</div>
          <div className="text-[11px] text-wms-success mt-0.5">
            <TrendingUp className="size-3 inline" /> +12.4% vs 上月
          </div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground flex items-center gap-1">
            <Clock data-icon="inline-start" /> 总工时
          </div>
          <div className="text-2xl font-bold mt-1">126.8 <span className="text-sm font-normal">h</span></div>
          <div className="text-[11px] text-muted-foreground mt-0.5">31 个工作日</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground flex items-center gap-1">
            <Users data-icon="inline-start" /> 平均效率
          </div>
          <div className="text-2xl font-bold mt-1">226 <span className="text-sm font-normal text-muted-foreground">件/h</span></div>
          <div className="text-[11px] text-wms-success mt-0.5">
            <TrendingUp className="size-3 inline" /> +8.2%
          </div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">异常率</div>
          <div className="text-2xl font-bold mt-1 text-wms-warning">2.1%</div>
          <div className="text-[11px] text-muted-foreground mt-0.5">28 / 1340 单</div>
        </Card>
      </div>

      {/* 排行 */}
      <div className="px-6 py-4 border-b">
        <div className="text-sm font-semibold mb-2">月度效率排行</div>
        <div className="grid grid-cols-4 gap-3">
          {[
            { rank: 1, name: "张三 (u001)", eff: 290, color: "text-wms-success" },
            { rank: 2, name: "王五 (u003)", eff: 219, color: "text-wms-success" },
            { rank: 3, name: "李四 (u002)", eff: 227, color: "" },
            { rank: 4, name: "赵六 (u004)", eff: 162, color: "text-wms-warning" },
          ].map((p) => (
            <Card key={p.rank} className="p-3 flex items-center gap-3">
              <div className={`text-2xl font-bold ${p.color}`}>#{p.rank}</div>
              <div className="flex-1">
                <div className="text-sm font-medium">{p.name}</div>
                <div className="text-xs text-muted-foreground">平均 {p.eff} 件/h</div>
              </div>
            </Card>
          ))}
        </div>
      </div>

      {/* 明细 */}
      <div className="px-6 py-4">
        <div className="text-sm font-semibold mb-2">日度明细（最近 3 天）</div>
        <DataTable columns={cols} data={MOCK} rowKey={(r) => `${r.date}-${r.worker}`} />
        <div className="mt-3 text-xs text-muted-foreground">共 124 条 · 显示 8 条 · <a className="text-primary underline cursor-pointer">查看全部</a></div>
      </div>
    </div>
  );
}
