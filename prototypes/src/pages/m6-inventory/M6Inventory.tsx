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
import { Search, Download, FileSpreadsheet, FileText, Database, Shield } from "lucide-react";

interface InvCheckRow {
  date: string;
  st: string;
  zone: string;
  worker: string;
  bookQty: number;
  realQty: number;
  diff: number;
  diffRate: string;
}

const INV_DATA: InvCheckRow[] = [
  { date: "2026-04-05", st: "ST-2026-0405", zone: "A 常温区", worker: "张/李", bookQty: 4820, realQty: 4818, diff: -2, diffRate: "-0.04%" },
  { date: "2026-04-12", st: "ST-2026-0412", zone: "C 冷藏区", worker: "王/赵", bookQty: 1240, realQty: 1240, diff: 0, diffRate: "0.00%" },
  { date: "2026-04-19", st: "ST-2026-0419", zone: "Q 隔离区", worker: "张/钱", bookQty: 86, realQty: 88, diff: +2, diffRate: "+2.33%" },
  { date: "2026-04-26", st: "ST-2026-0426", zone: "B 阴凉区", worker: "李/孙", bookQty: 2480, realQty: 2476, diff: -4, diffRate: "-0.16%" },
  { date: "2026-04-30", st: "ST-2026-0430", zone: "D 冷冻区", worker: "王/周", bookQty: 380, realQty: 380, diff: 0, diffRate: "0.00%" },
];

/**
 * M6Inventory — M6-002c 库存盘点月报
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP §95 库存盘点月报 / 双盘记录 / 盈亏分析）
 * Wave：Wave 3.0
 * 业务约束：每月至少 1 次全盘 + 麻醉药每月必盘；盘亏 > 1% 触发主管审核；留 5 年
 *
 * @example
 *   <M6Inventory />
 */
export function M6Inventory() {
  const cols: DataTableColumn<InvCheckRow>[] = [
    { key: "date", header: "盘点日期", render: (r) => <span className="font-mono text-xs">{r.date}</span> },
    { key: "st", header: "盘点单号", render: (r) => <span className="font-mono text-xs text-primary">{r.st}</span> },
    { key: "zone", header: "盘点区域", render: (r) => <span className="text-sm">{r.zone}</span> },
    { key: "worker", header: "双盘人员", render: (r) => <span className="text-xs">{r.worker}</span> },
    { key: "book", header: "账面数", align: "right", render: (r) => <span className="text-sm">{r.bookQty}</span> },
    { key: "real", header: "实盘数", align: "right", render: (r) => <span className="text-sm font-medium">{r.realQty}</span> },
    { key: "diff", header: "差异", align: "right", render: (r) => {
      const color = r.diff < 0 ? "text-destructive" : r.diff > 0 ? "text-wms-warning" : "text-muted-foreground";
      return <span className={`text-sm font-semibold ${color}`}>{r.diff > 0 ? "+" : ""}{r.diff}</span>;
    }},
    { key: "rate", header: "差异率", align: "right", render: (r) => {
      const isAbnormal = r.diffRate.startsWith("+2") || r.diffRate.startsWith("-1");
      return <span className={`text-xs font-mono ${isAbnormal ? "text-wms-warning" : ""}`}>{r.diffRate}</span>;
    }},
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="库存盘点月报"
        subtitle="M6-002c · GSP §95 · 月度盘点 + 双盘记录 + 盈亏分析"
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
        <div><label className="text-xs text-muted-foreground mb-1 block">盘点区域</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="A">A 常温区</SelectItem>
            <SelectItem value="B">B 阴凉区</SelectItem>
            <SelectItem value="C">C 冷藏区</SelectItem>
            <SelectItem value="D">D 冷冻区</SelectItem>
            <SelectItem value="Q">Q 隔离区</SelectItem>
          </SelectContent></Select></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">差异档</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="abnormal">异常（&gt; 1%）</SelectItem>
            <SelectItem value="zero">无差异</SelectItem>
          </SelectContent></Select></div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search className="h-3.5 w-3.5 mr-1" /> 查询</Button>
        </div>
      </div>

      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">盘点单数</div>
          <div className="text-xl font-bold mt-1">12</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">总盘点件数</div>
          <div className="text-xl font-bold mt-1">9,006</div>
        </Card>
        <Card className="p-3 border-destructive/40 bg-destructive/5">
          <div className="text-xs text-destructive">盘亏</div>
          <div className="text-xl font-bold mt-1 text-destructive">-6</div>
          <div className="text-[11px] text-destructive">-0.07%</div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning">盘盈</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">+2</div>
          <div className="text-[11px] text-wms-warning">+0.02%</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">差异 &gt; 1%</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">1</div>
          <div className="text-[11px] text-muted-foreground">需主管审核</div>
        </Card>
      </div>

      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">月度盘点汇总（2026-04）</div>
            <span className="text-xs text-muted-foreground font-mono">MD5: 7b2a…e1c9</span>
          </div>
          <DataTable columns={cols} data={INV_DATA} rowKey={(r) => r.st} />
          <div className="mt-3 text-xs text-muted-foreground">本月共 12 次盘点（4 次全盘 + 8 次循环盘点）</div>
        </div>
        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download className="h-4 w-4" /> 导出
            </div>
            <div className="space-y-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText className="h-4 w-4 mr-2" /> PDF（含双盘签字）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet className="h-4 w-4 mr-2" /> Excel
              </Button>
            </div>
          </Card>
          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield className="h-4 w-4" /> GSP §95 合规
            </div>
            <ul className="text-xs text-muted-foreground space-y-1">
              <li>· 每月至少 1 次全盘</li>
              <li>· 麻醉药/精神药每月必盘</li>
              <li>· 双盘记录强制</li>
              <li>· 盘亏 &gt; 1% 主管审核</li>
              <li>· 留存 5 年</li>
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
