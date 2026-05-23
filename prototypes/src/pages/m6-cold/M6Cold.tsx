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
  TempChart,
  type DataTableColumn,
  type TempPoint,
} from "@wms/ui";
import { Search, Download, FileText, FileSpreadsheet, Database, Shield, Snowflake } from "lucide-react";

interface ExcursionRow {
  time: string;
  zone: string;
  duration: string;
  peak: string;
  threshold: string;
  registered: boolean;
  notified: boolean;
}

const EXCURSION_DATA: ExcursionRow[] = [
  { time: "2026-04-03 14:23", zone: "B 阴凉区", duration: "18 min", peak: "21.2℃", threshold: "≤20℃", registered: true, notified: true },
  { time: "2026-04-12 22:45", zone: "D 冷冻区", duration: "8 min", peak: "-14.6℃", threshold: "≤-15℃", registered: true, notified: true },
  { time: "2026-04-18 09:08", zone: "C 冷藏区", duration: "3 min", peak: "9.1℃", threshold: "2-8℃", registered: false, notified: false },
];

// 月度温度分布趋势（30 天每天 1 个点）
function genMonthly(base: number, amp: number): TempPoint[] {
  return Array.from({ length: 30 }, (_, i) => ({
    t: `04-${String(i + 1).padStart(2, "0")}`,
    v: Math.round((base + Math.sin(i / 5) * amp + (Math.random() - 0.5) * 0.5) * 10) / 10,
  }));
}
const COLD_MONTHLY: TempPoint[] = genMonthly(5, 1.2);

/**
 * M6Cold — M6-002d 冷链温度月报
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP §64 冷链温度月报 / 偏差登记 / 全程合规率）
 * Wave：Wave 3.0
 * 业务约束：偏差需登记 + 通知货主；冷链全程台账留 5 年；监管 EDI 包含冷链摘要
 *
 * @example
 *   <M6Cold />
 */
export function M6Cold() {
  const cols: DataTableColumn<ExcursionRow>[] = [
    { key: "time", header: "时间", render: (r) => <span className="font-mono text-xs">{r.time}</span> },
    { key: "zone", header: "温区", render: (r) => <span className="text-sm">{r.zone}</span> },
    { key: "duration", header: "持续时长", align: "right", render: (r) => <span className="text-sm">{r.duration}</span> },
    { key: "peak", header: "峰值", align: "right", render: (r) => <span className="text-sm font-mono text-wms-warning">{r.peak}</span> },
    { key: "threshold", header: "阈值", align: "right", render: (r) => <span className="text-xs text-muted-foreground">{r.threshold}</span> },
    { key: "registered", header: "已登记", render: (r) =>
      r.registered ? <span className="text-xs text-wms-success">✓</span> : <span className="text-xs text-destructive">✗ 未登记</span>,
    },
    { key: "notified", header: "已通知货主", render: (r) =>
      r.notified ? <span className="text-xs text-wms-success">✓</span> : <span className="text-xs text-destructive">✗ 未通知</span>,
    },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="冷链温度月报"
        subtitle="M6-002d · GSP §64 · 月度温度记录 + 偏差登记 + 全程合规率"
        actions={
          <Button variant="outline" size="sm">
            <Database className="h-4 w-4 mr-1" /> 数据签名
          </Button>
        }
      />

      <div className="px-6 py-4 border-b bg-muted/30 grid grid-cols-5 gap-3 items-end">
        <div><label className="text-xs text-muted-foreground mb-1 block">起始日期</label>
          <Input type="date" defaultValue="2026-04-01" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">截止日期</label>
          <Input type="date" defaultValue="2026-04-30" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">温区</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部冷链区</SelectItem>
            <SelectItem value="C">C 冷藏区</SelectItem>
            <SelectItem value="D">D 冷冻区</SelectItem>
            <SelectItem value="B">B 阴凉区</SelectItem>
          </SelectContent></Select></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">仅显示偏差</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="excursion">仅偏差</SelectItem>
            <SelectItem value="unregistered">未登记偏差</SelectItem>
          </SelectContent></Select></div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search className="h-3.5 w-3.5 mr-1" /> 查询</Button>
        </div>
      </div>

      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">采样点数</div>
          <div className="text-xl font-bold mt-1">86,400</div>
          <div className="text-[11px] text-muted-foreground">30 天 × 6 区 × 5min</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">平均温度</div>
          <div className="text-xl font-bold mt-1 text-wms-cold">5.2 <span className="text-sm font-normal">℃</span></div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning">偏差次数</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">3</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">已登记 / 已通知</div>
          <div className="text-xl font-bold mt-1 text-wms-success">2/2</div>
          <div className="text-[11px] text-destructive">未登记 1</div>
        </Card>
        <Card className="p-3 border-wms-success/40 bg-wms-success/5">
          <div className="text-xs text-wms-success">全程合规率</div>
          <div className="text-xl font-bold mt-1 text-wms-success">99.97%</div>
        </Card>
      </div>

      <div className="px-6 py-4 border-b">
        <div className="flex items-center justify-between mb-3">
          <div>
            <div className="text-sm font-semibold flex items-center gap-2">
              <Snowflake className="h-4 w-4 text-wms-cold" /> C 冷藏区 · 月度温度趋势
            </div>
            <div className="text-xs text-muted-foreground mt-0.5">每日均值 30 个 · 阈值 2-8℃</div>
          </div>
        </div>
        <TempChart points={COLD_MONTHLY} minThreshold={2} maxThreshold={8} height={200} />
      </div>

      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">本月偏差记录（3 条）</div>
            <span className="text-xs text-muted-foreground font-mono">MD5: 9c8b…a4d3</span>
          </div>
          <DataTable columns={cols} data={EXCURSION_DATA} rowKey={(r) => r.time} />
        </div>
        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download className="h-4 w-4" /> 导出
            </div>
            <div className="space-y-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText className="h-4 w-4 mr-2" /> PDF（含温度曲线）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet className="h-4 w-4 mr-2" /> Excel（含原始 86400 点）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <Database className="h-4 w-4 mr-2" /> JSON（药监 EDI）
              </Button>
            </div>
          </Card>
          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield className="h-4 w-4" /> GSP §64 合规
            </div>
            <ul className="text-xs text-muted-foreground space-y-1">
              <li>· 每 5 分钟采样</li>
              <li>· 偏差登记 + 通知货主</li>
              <li>· 全程台账留 5 年</li>
              <li>· EDI 报送药监</li>
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
