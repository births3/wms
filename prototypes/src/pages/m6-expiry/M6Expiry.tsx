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
  StatusBadge,
  type DataTableColumn,
} from "@wms/ui";
import { Search, Download, FileText, FileSpreadsheet, Database, Shield, AlertTriangle } from "lucide-react";

interface ExpiryRow {
  itemCode: string;
  itemName: string;
  batch: string;
  expiry: string;
  daysToExpiry: number;
  qty: number;
  location: string;
  category: "near_expiry" | "expired" | "isolated";
}

const EXPIRY_DATA: ExpiryRow[] = [
  { itemCode: "P-003045", itemName: "复方甘草口服溶液 100ml", batch: "20240801", expiry: "2026-08-01",
    daysToExpiry: 71, qty: 36, location: "A-02-04-12", category: "near_expiry" },
  { itemCode: "P-005120", itemName: "维生素 C 片 0.1g × 100", batch: "20240315", expiry: "2026-09-15",
    daysToExpiry: 116, qty: 240, location: "A-03-08-04", category: "near_expiry" },
  { itemCode: "P-003045", itemName: "复方甘草口服溶液 100ml", batch: "20231201", expiry: "2026-04-01",
    daysToExpiry: -50, qty: 8, location: "R-01-01-01", category: "expired" },
  { itemCode: "P-007890", itemName: "感冒灵颗粒 10g × 9", batch: "20231101", expiry: "2026-03-01",
    daysToExpiry: -82, qty: 24, location: "R-01-02-03", category: "expired" },
  { itemCode: "P-002134", itemName: "硝苯地平片 10mg × 100", batch: "20260301X", expiry: "2027-03-01",
    daysToExpiry: 283, qty: 12, location: "Q-02-01-04", category: "isolated" },
];

/**
 * M6Expiry — M6-002e 近效期/不合格品月报
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP §50 近效期月报 / 不合格品 / 销毁记录）
 * Wave：Wave 3.0
 * 业务约束：≤ 90 天预警；过期自动隔离；销毁需双人签字 + 监督
 *
 * @example
 *   <M6Expiry />
 */
export function M6Expiry() {
  const cols: DataTableColumn<ExpiryRow>[] = [
    { key: "code", header: "商品",
      render: (r) => (
        <div>
          <div className="font-mono text-xs text-primary">{r.itemCode}</div>
          <div className="text-sm">{r.itemName}</div>
        </div>
      ),
    },
    { key: "batch", header: "批号 / 效期", render: (r) => (
      <div className="text-xs">
        <div className="font-mono">{r.batch}</div>
        <div className={`mt-0.5 ${
          r.daysToExpiry < 0 ? "text-destructive font-medium" :
          r.daysToExpiry <= 90 ? "text-wms-warning font-medium" : "text-muted-foreground"
        }`}>
          {r.expiry} ({r.daysToExpiry < 0 ? `已过期 ${-r.daysToExpiry} 天` : `剩 ${r.daysToExpiry} 天`})
        </div>
      </div>
    )},
    { key: "qty", header: "库存", align: "right", render: (r) => <span className="text-sm font-medium">{r.qty}</span> },
    { key: "location", header: "库位", render: (r) => <span className="font-mono text-xs">{r.location}</span> },
    { key: "category", header: "类型", render: (r) => {
      if (r.category === "near_expiry") return <StatusBadge status="near_expiry" size="sm" label="近效期" />;
      if (r.category === "expired") return <StatusBadge status="expired" size="sm" label="已过期" />;
      return <StatusBadge status="isolated" size="sm" label="不合格隔离" />;
    }},
    { key: "actions", header: "操作", render: () => (
      <div className="flex gap-1">
        <Button variant="ghost" size="sm" className="h-7 px-2 text-xs">详情</Button>
        <Button variant="ghost" size="sm" className="h-7 px-2 text-xs text-destructive">销毁</Button>
      </div>
    )},
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="近效期/不合格品月报"
        subtitle="M6-002e · GSP §50 · 月度近效期 + 已过期 + 隔离品 + 销毁记录"
        actions={
          <Button variant="outline" size="sm">
            <Database data-icon="inline-start" /> 数据签名
          </Button>
        }
      />

      {/* 提醒 banner */}
      <div className="mx-6 mt-4 p-3 bg-destructive/10 border border-destructive/30 rounded-md flex items-start gap-2">
        <AlertTriangle className="size-4 text-destructive flex-shrink-0 mt-0.5" />
        <div className="text-xs flex-1">
          <span className="font-medium text-destructive">2 个批次已过期需销毁</span>
          <span className="text-muted-foreground ml-2">
            P-003045 批 20231201（8 件）+ P-007890 批 20231101（24 件）· 等待双人销毁签字
          </span>
        </div>
      </div>

      <div className="px-6 py-4 border-b bg-muted/30 grid grid-cols-5 gap-3 items-end">
        <div><label className="text-xs text-muted-foreground mb-1 block">起始日期</label>
          <Input type="date" defaultValue="2026-04-01" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">截止日期</label>
          <Input type="date" defaultValue="2026-04-30" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">类型</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="near_expiry">近效期</SelectItem>
            <SelectItem value="expired">已过期</SelectItem>
            <SelectItem value="isolated">隔离</SelectItem>
          </SelectContent></Select></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">效期阈值</label>
          <Select defaultValue="90"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="30">≤ 30 天</SelectItem>
            <SelectItem value="60">≤ 60 天</SelectItem>
            <SelectItem value="90">≤ 90 天（GSP）</SelectItem>
            <SelectItem value="180">≤ 180 天</SelectItem>
          </SelectContent></Select></div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search data-icon="inline-start" /> 查询</Button>
        </div>
      </div>

      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">总品规</div>
          <div className="text-xl font-bold mt-1">2,348</div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning">近效期 ≤90 天</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">42</div>
        </Card>
        <Card className="p-3 border-destructive/40 bg-destructive/5">
          <div className="text-xs text-destructive">已过期</div>
          <div className="text-xl font-bold mt-1 text-destructive">3</div>
        </Card>
        <Card className="p-3 border-muted-foreground/30 bg-muted/30">
          <div className="text-xs text-muted-foreground">隔离/不合格</div>
          <div className="text-xl font-bold mt-1">8</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">本月销毁</div>
          <div className="text-xl font-bold mt-1">2</div>
          <div className="text-[11px] text-muted-foreground">含双人签字</div>
        </Card>
      </div>

      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">明细（按效期升序）</div>
            <span className="text-xs text-muted-foreground font-mono">MD5: 4e7d…b1a6</span>
          </div>
          <DataTable columns={cols} data={EXPIRY_DATA} rowKey={(r) => `${r.itemCode}-${r.batch}`} />
          <div className="mt-3 text-xs text-muted-foreground">显示 5 / 53 条</div>
        </div>
        <div className="flex flex-col gap-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download data-icon="inline-start" /> 导出
            </div>
            <div className="flex flex-col gap-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText data-icon="inline-start" /> PDF
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet data-icon="inline-start" /> Excel
              </Button>
            </div>
          </Card>
          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield data-icon="inline-start" /> GSP §50 合规
            </div>
            <ul className="text-xs text-muted-foreground flex flex-col gap-1">
              <li>· ≤ 90 天预警</li>
              <li>· 过期自动隔离</li>
              <li>· 销毁双人签字</li>
              <li>· 不合格品红区独立</li>
              <li>· 留存 5 年</li>
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
