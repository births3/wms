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
  StatusBadge,
  PageHeader,
  DataTable,
  type DataTableColumn,
} from "@wms/ui";
import { Search, Download, AlertTriangle, Package, Snowflake, Lock } from "lucide-react";

interface InvRow {
  id: string;
  itemCode: string;
  itemName: string;
  spec: string;
  batch: string;
  expiry: string;
  /** 库存数 */
  qty: number;
  /** 占用（拣货单等） */
  reserved: number;
  /** 可用 */
  available: number;
  location: string;
  zone: "RT" | "CR" | "CL" | "FR";
  status: "normal" | "near_expiry" | "isolated" | "expired";
  daysToExpiry: number;
}

const MOCK: InvRow[] = [
  { id: "1", itemCode: "P-001234", itemName: "葡萄糖注射液", spec: "500ml × 24",
    batch: "20250901A", expiry: "2027-09-01", qty: 480, reserved: 12, available: 468,
    location: "A-01-02-03", zone: "RT", status: "normal", daysToExpiry: 467 },
  { id: "2", itemCode: "P-001234", itemName: "葡萄糖注射液", spec: "500ml × 24",
    batch: "20260301A", expiry: "2028-03-01", qty: 240, reserved: 8, available: 232,
    location: "A-01-02-04", zone: "RT", status: "normal", daysToExpiry: 648 },
  { id: "3", itemCode: "P-001235", itemName: "重组人胰岛素", spec: "3ml:300IU × 5",
    batch: "20260315B", expiry: "2027-03-15", qty: 60, reserved: 5, available: 55,
    location: "C-01-01-08", zone: "CR", status: "normal", daysToExpiry: 297 },
  { id: "4", itemCode: "P-002001", itemName: "盐酸吗啡片", spec: "10mg × 100",
    batch: "20260101N", expiry: "2027-01-01", qty: 24, reserved: 0, available: 24,
    location: "Q-01-01-01", zone: "RT", status: "isolated", daysToExpiry: 224 },
  { id: "5", itemCode: "P-003045", itemName: "复方甘草口服溶液", spec: "100ml × 1",
    batch: "20240801", expiry: "2026-08-01", qty: 36, reserved: 0, available: 36,
    location: "A-02-04-12", zone: "RT", status: "near_expiry", daysToExpiry: 71 },
  { id: "6", itemCode: "P-004088", itemName: "辉瑞疫苗（示例）", spec: "0.3ml × 6",
    batch: "20260201F", expiry: "2027-03-01", qty: 12, reserved: 0, available: 12,
    location: "D-01-01-01", zone: "FR", status: "normal", daysToExpiry: 283 },
  { id: "7", itemCode: "P-003045", itemName: "复方甘草口服溶液", spec: "100ml × 1",
    batch: "20231201", expiry: "2026-04-01", qty: 8, reserved: 0, available: 0,
    location: "R-01-01-01", zone: "RT", status: "expired", daysToExpiry: -50 },
];

const ZONE_LABEL = {
  RT: { text: "常温", color: "bg-muted text-muted-foreground" },
  CR: { text: "冷藏", color: "bg-wms-cold/10 text-wms-cold" },
  CL: { text: "阴凉", color: "bg-wms-cold/5 text-wms-cold/70" },
  FR: { text: "冷冻", color: "bg-wms-cold/20 text-wms-cold font-medium" },
};

/**
 * M3Inventory — M3-001 库存查询
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M3-001（库存多维查询 / 批号 / 效期 / 库位 / 状态）
 * Wave：Wave 2.5（M3 库存核心）
 * 业务约束：不合格品/隔离品独立库位；近效期 90 天预警；负库存严禁
 *
 * @example
 *   <M3Inventory />
 */
export function M3Inventory() {
  const cols: DataTableColumn<InvRow>[] = [
    {
      key: "code", header: "商品",
      render: (r) => (
        <div>
          <div className="font-mono text-xs text-primary">{r.itemCode}</div>
          <div className="text-sm font-medium mt-0.5">{r.itemName}</div>
          <div className="text-xs text-muted-foreground">{r.spec}</div>
        </div>
      ),
    },
    {
      key: "batch", header: "批号 / 效期",
      render: (r) => (
        <div className="text-xs">
          <div className="font-mono">{r.batch}</div>
          <div className={`mt-0.5 ${
            r.status === "expired" ? "text-destructive font-medium" :
            r.status === "near_expiry" ? "text-wms-warning font-medium" : "text-muted-foreground"
          }`}>
            {r.expiry} ({r.daysToExpiry < 0 ? `已过期 ${-r.daysToExpiry} 天` : `剩 ${r.daysToExpiry} 天`})
          </div>
        </div>
      ),
    },
    {
      key: "location", header: "库位",
      render: (r) => {
        const meta = ZONE_LABEL[r.zone];
        return (
          <div className="text-xs">
            <div className="font-mono">{r.location}</div>
            <span className={`text-[10px] px-1 py-0.5 rounded mt-0.5 inline-block ${meta.color}`}>{meta.text}</span>
          </div>
        );
      },
    },
    {
      key: "qty", header: "库存数",
      align: "right",
      render: (r) => (
        <div className="text-right">
          <div className="text-sm font-semibold">{r.qty}</div>
          <div className="text-[11px] text-muted-foreground">瓶</div>
        </div>
      ),
    },
    {
      key: "reserved", header: "占用",
      align: "right",
      render: (r) => (
        <div className="text-sm text-right text-muted-foreground">{r.reserved}</div>
      ),
    },
    {
      key: "available", header: "可用",
      align: "right",
      render: (r) => (
        <div className={`text-right text-sm font-semibold ${
          r.available === 0 ? "text-destructive" : ""
        }`}>{r.available}</div>
      ),
    },
    {
      key: "status", header: "状态",
      render: (r) => {
        if (r.status === "expired") return <StatusBadge status="expired" size="sm" />;
        if (r.status === "near_expiry") return <StatusBadge status="near_expiry" size="sm" />;
        if (r.status === "isolated") return <StatusBadge status="isolated" size="sm" label="隔离" />;
        return <StatusBadge status="qualified" size="sm" label="正常" />;
      },
    },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="库存查询"
        subtitle="M3-001 · 多维筛选 + 实时占用 + 效期预警"
        actions={
          <>
            <Button size="sm" variant="ghost" onClick={() => (window.location.hash = "#m6-custom")} title="另存为自定义报表">
              ⇲ 另存为报表
            </Button>
            <Button variant="outline" size="sm">
              <Download data-icon="inline-start" /> 导出
            </Button>
            <Button size="sm">高级查询</Button>
          </>
        }
      />

      {/* 统计卡片 */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground flex items-center gap-1">
            <Package data-icon="inline-start" /> 总品规
          </div>
          <div className="text-xl font-bold mt-1">2,348</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">总库存</div>
          <div className="text-xl font-bold mt-1">186,420 <span className="text-xs font-normal text-muted-foreground">瓶</span></div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning flex items-center gap-1">
            <AlertTriangle data-icon="inline-start" /> 近效期
          </div>
          <div className="text-xl font-bold mt-1 text-wms-warning">42</div>
        </Card>
        <Card className="p-3 border-destructive/40 bg-destructive/5">
          <div className="text-xs text-destructive flex items-center gap-1">
            <Lock data-icon="inline-start" /> 隔离/不合格
          </div>
          <div className="text-xl font-bold mt-1 text-destructive">8</div>
        </Card>
        <Card className="p-3 border-wms-cold/40 bg-wms-cold/5">
          <div className="text-xs text-wms-cold flex items-center gap-1">
            <Snowflake data-icon="inline-start" /> 冷链占比
          </div>
          <div className="text-xl font-bold mt-1 text-wms-cold">7.9%</div>
        </Card>
      </div>

      {/* 筛选 */}
      <div className="px-6 py-4 border-b grid grid-cols-6 gap-3 items-end">
        <div className="col-span-2">
          <label className="text-xs text-muted-foreground mb-1 block">关键字</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input className="pl-9" placeholder="品名 / 编码 / 批号 / 库位" />
          </div>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">温区</label>
          <Select>
            <SelectTrigger><SelectValue placeholder="全部" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="rt">常温</SelectItem>
              <SelectItem value="cr">冷藏</SelectItem>
              <SelectItem value="fr">冷冻</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">效期</label>
          <Select>
            <SelectTrigger><SelectValue placeholder="全部" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="near">≤ 90 天</SelectItem>
              <SelectItem value="expired">已过期</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">状态</label>
          <Select>
            <SelectTrigger><SelectValue placeholder="全部" /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="normal">正常</SelectItem>
              <SelectItem value="isolated">隔离</SelectItem>
              <SelectItem value="expired">过期</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      <DataTable columns={cols} data={MOCK} rowKey={(r) => r.id} />

      <Card className="mx-6 my-4 p-3 bg-muted/30">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">每页 50 条 · 共 4,628 条 · 第 1/93 页</span>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" disabled>上一页</Button>
            <Button variant="outline" size="sm">下一页</Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
