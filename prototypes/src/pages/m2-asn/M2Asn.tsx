import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from "@/components/ui/select";
import {
  PageHeader,
  DataTable,
  StatusBadge,
  type DataTableColumn,
} from "@/components/business";
import { Plus, Search, Upload, Download, FileWarning } from "lucide-react";

interface AsnRow {
  asn: string;
  poNo: string;
  supplier: string;
  itemCount: number;
  totalQty: number;
  amount: string;
  arrivalAt: string;
  isCold: boolean;
  status: "draft" | "submitted" | "received" | "rejected";
}

const MOCK: AsnRow[] = [
  { asn: "PO-2026-0034", poNo: "PUR-A4521", supplier: "国药控股北京", itemCount: 4, totalQty: 320, amount: "32,000", arrivalAt: "今日 14:30", isCold: true, status: "submitted" },
  { asn: "PO-2026-0035", poNo: "PUR-A4522", supplier: "上海医药华东", itemCount: 8, totalQty: 1820, amount: "215,400", arrivalAt: "今日 09:15", isCold: false, status: "received" },
  { asn: "PO-2026-0036", poNo: "PUR-A4523", supplier: "九州通医药", itemCount: 12, totalQty: 3640, amount: "108,200", arrivalAt: "明日 10:00", isCold: false, status: "draft" },
  { asn: "PO-2026-0037", poNo: "PUR-A4524", supplier: "甘李药业", itemCount: 3, totalQty: 60, amount: "29,160", arrivalAt: "今日 17:45", isCold: true, status: "submitted" },
  { asn: "PO-2026-0033", poNo: "PUR-A4520", supplier: "XX 医药贸易", itemCount: 1, totalQty: 240, amount: "8,640", arrivalAt: "昨日 16:20", isCold: false, status: "rejected" },
];

const STATUS_MAP = {
  draft: { status: "pending" as const, label: "草稿" },
  submitted: { status: "in_progress" as const, label: "待收货" },
  received: { status: "completed" as const, label: "已入库" },
  rejected: { status: "unqualified" as const, label: "已拒收" },
};

/**
 * M2Asn — M2-001 ASN 接收（PC 入口）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M2-001（采购员上传/接收 ASN / 触发 PDA 收货任务）
 * Wave：Wave 2.5（M2 PC 流程入口）
 * 业务约束：ASN 必含批号效期生产企业；冷链单独标记；提交后下发 PDA M2-002
 *
 * @example
 *   <M2Asn />
 */
export function M2Asn() {
  const cols: DataTableColumn<AsnRow>[] = [
    { key: "asn", header: "ASN 号",
      render: (r) => <span className="font-mono text-xs text-primary">{r.asn}</span> },
    { key: "po", header: "采购单号",
      render: (r) => <span className="font-mono text-xs">{r.poNo}</span> },
    { key: "supplier", header: "供应商",
      render: (r) => <span className="text-sm">{r.supplier}</span> },
    { key: "items", header: "明细",
      render: (r) => (
        <div className="text-xs">
          <span className="font-medium">{r.itemCount} 项</span>
          <span className="text-muted-foreground ml-1.5">{r.totalQty} 件</span>
          {r.isCold && <span className="ml-1.5 text-[10px] px-1 py-0.5 bg-wms-cold/10 text-wms-cold rounded">❄️ 冷链</span>}
        </div>
      ) },
    { key: "amount", header: "金额", align: "right",
      render: (r) => <span className="font-mono text-sm">¥{r.amount}</span> },
    { key: "arrival", header: "预计到货", render: (r) => <span className="text-xs">{r.arrivalAt}</span> },
    { key: "status", header: "状态",
      render: (r) => <StatusBadge status={STATUS_MAP[r.status].status} size="sm" label={STATUS_MAP[r.status].label} /> },
    { key: "actions", header: "操作",
      render: (r) => (
        <div className="flex gap-1">
          <Button variant="ghost" size="sm" className="h-7 px-2 text-xs">详情</Button>
          {r.status === "submitted" && (
            <Button variant="ghost" size="sm" className="h-7 px-2 text-xs">下发 PDA</Button>
          )}
        </div>
      ) },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="ASN 接收单"
        subtitle="M2-001 · 采购入库流程入口 · 触发 PDA 收货任务 · GSP §82 ASN 准入"
        actions={
          <>
            <Button variant="outline" size="sm">
              <Upload className="h-4 w-4 mr-1" /> 导入 EDI
            </Button>
            <Button variant="outline" size="sm">
              <Download className="h-4 w-4 mr-1" /> 导出
            </Button>
            <Button size="sm">
              <Plus className="h-4 w-4 mr-1" /> 新建 ASN
            </Button>
          </>
        }
      />

      {/* 提醒 */}
      <div className="mx-6 mt-4 p-3 bg-wms-warning/10 border border-wms-warning/30 rounded-md flex items-start gap-2">
        <FileWarning className="h-4 w-4 text-wms-warning flex-shrink-0 mt-0.5" />
        <div className="text-xs flex-1">
          <span className="font-medium text-wms-warning">2 张 ASN 提交超 24 小时未收货</span>
          <span className="text-muted-foreground ml-2">PO-2026-0026 / PO-2026-0029 · 请联系供应商或下发 PDA</span>
        </div>
      </div>

      {/* 筛选 */}
      <div className="px-6 py-4 grid grid-cols-6 gap-3 items-end">
        <div className="col-span-2">
          <label className="text-xs text-muted-foreground mb-1 block">关键字</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <Input className="pl-9" placeholder="ASN / 采购单号 / 供应商" />
          </div>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">状态</label>
          <Select defaultValue="all">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="submitted">待收货</SelectItem>
              <SelectItem value="received">已入库</SelectItem>
              <SelectItem value="rejected">已拒收</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">冷链</label>
          <Select defaultValue="all">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="cold">仅冷链</SelectItem>
              <SelectItem value="normal">仅常温</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">日期范围</label>
          <Input type="date" defaultValue="2026-05-22" />
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      {/* 统计 */}
      <div className="px-6 py-3 border-y bg-muted/30 flex items-center gap-6 text-xs text-muted-foreground">
        <span>共 <span className="font-semibold text-foreground">86</span> 张</span>
        <span>待收货 <span className="font-semibold text-primary">12</span></span>
        <span>已入库 <span className="font-semibold text-wms-success">68</span></span>
        <span>已拒收 <span className="font-semibold text-destructive">3</span></span>
        <span>冷链占 <span className="font-semibold text-wms-cold">18%</span></span>
      </div>

      <DataTable columns={cols} data={MOCK} rowKey={(r) => r.asn} />

      <Card className="mx-6 my-4 p-3 bg-muted/30">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">每页 50 条 · 共 86 条 · 第 1/2 页</span>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" disabled>上一页</Button>
            <Button variant="outline" size="sm">下一页</Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
