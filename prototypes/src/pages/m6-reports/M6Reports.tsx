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
import { useState } from "react";
import {
  PageHeader,
  DataTable,
  AuditTimeline,
  type DataTableColumn,
  type AuditTimelineEvent,
} from "@/components/business";
import { Search, Download, FileSpreadsheet, FileText, Database, Shield } from "lucide-react";

/**
 * M6Reports — M6-002 GSP 法定报表
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP 法定报表查询 + 导出 / 多维度筛选 / 含数据签名）
 * Wave：Wave 3.0（M6 报表合规）
 * 业务约束：导出含 MD5 签名（防篡改）；保留 5 年；查询/导出全部写 H2 审计
 *
 * @example
 *   <M6Reports />
 */

interface PurchaseRow {
  date: string;
  asn: string;
  supplier: string;
  itemName: string;
  qty: number;
  amount: string;
  inspector: string;
  status: "qualified" | "partial" | "rejected";
}

const PURCHASE_DATA: PurchaseRow[] = [
  { date: "2026-04-02", asn: "PO-2026-0001", supplier: "国药控股北京", itemName: "葡萄糖注射液 500ml × 24",
    qty: 240, amount: "23,520.00", inspector: "张三 (u001)", status: "qualified" },
  { date: "2026-04-05", asn: "PO-2026-0008", supplier: "上海医药华东", itemName: "重组人胰岛素 3ml × 5",
    qty: 60, amount: "29,160.00", inspector: "李四 (u002)", status: "qualified" },
  { date: "2026-04-08", asn: "PO-2026-0012", supplier: "九州通医药", itemName: "复方甘草口服溶液 100ml",
    qty: 36, amount: "1,296.00", inspector: "王五 (u003)", status: "partial" },
  { date: "2026-04-15", asn: "PO-2026-0019", supplier: "甘李药业", itemName: "甘精胰岛素 3ml × 5",
    qty: 50, amount: "31,500.00", inspector: "张三 (u001)", status: "qualified" },
  { date: "2026-04-22", asn: "PO-2026-0027", supplier: "国药控股北京", itemName: "盐酸吗啡片 10mg × 100",
    qty: 24, amount: "7,680.00", inspector: "李四 (u002)", status: "qualified" },
  { date: "2026-04-28", asn: "PO-2026-0033", supplier: "XX 医药贸易", itemName: "复方甘草口服溶液 100ml",
    qty: 240, amount: "8,640.00", inspector: "赵六 (u004)", status: "rejected" },
];

const AUDIT_EVENTS: AuditTimelineEvent[] = [
  { id: "e1", time: "2026-05-23 02:25", actor: "system", action: "自动生成报表",
    module: "M6", resource: "采购入库月报 2026-04", status: "completed",
    detail: <div className="text-xs">含数据 MD5 签名 a3f2…b9x7 · 6 条记录</div> },
  { id: "e2", time: "2026-05-15 14:30", actor: "李四 (u002)", action: "下载 PDF",
    module: "M6", resource: "采购入库月报 2026-04", status: "qualified",
    detail: <div className="text-xs">IP 10.2.0.22 · 累计 3 次</div> },
  { id: "e3", time: "2026-05-15 10:08", actor: "张三 (u001)", action: "下载 Excel",
    module: "M6", resource: "采购入库月报 2026-04", status: "qualified",
    detail: <div className="text-xs">IP 10.2.0.18 · 累计 5 次</div> },
  { id: "e4", time: "2026-05-08 09:12", actor: "王五 (u003)", action: "推送 ERP（药监 EDI）",
    module: "M6", resource: "近效期月报 2026-04", status: "qualified",
    detail: <div className="text-xs">含数据签名 · ERP 转药监</div> },
];

const STATUS_LABEL = {
  qualified: { text: "合格入库", color: "bg-wms-success/10 text-wms-success" },
  partial: { text: "部分接收", color: "bg-wms-warning/10 text-wms-warning" },
  rejected: { text: "拒收", color: "bg-destructive/10 text-destructive" },
};

export function M6Reports() {
  const [expanded, setExpanded] = useState<string | undefined>("e1");

  const cols: DataTableColumn<PurchaseRow>[] = [
    { key: "date", header: "日期", render: (r) => <span className="font-mono text-xs">{r.date}</span> },
    { key: "asn", header: "ASN 号", render: (r) => <span className="font-mono text-xs text-primary">{r.asn}</span> },
    { key: "supplier", header: "供应商", render: (r) => <span className="text-sm">{r.supplier}</span> },
    { key: "item", header: "商品", render: (r) => <span className="text-sm">{r.itemName}</span> },
    { key: "qty", header: "数量", align: "right",
      render: (r) => <span className="text-sm font-medium">{r.qty}</span> },
    { key: "amount", header: "金额", align: "right",
      render: (r) => <span className="text-sm font-mono">¥{r.amount}</span> },
    { key: "inspector", header: "验收人", render: (r) => <span className="text-xs">{r.inspector}</span> },
    { key: "status", header: "状态", render: (r) => {
      const meta = STATUS_LABEL[r.status];
      return <span className={`text-xs px-1.5 py-0.5 rounded ${meta.color}`}>{meta.text}</span>;
    }},
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="GSP 法定报表"
        subtitle="M6-002 · 查询 + 导出 · 含数据签名 · 留存 5 年（GSP §95）"
        actions={
          <>
            <Button variant="outline" size="sm">
              <Database className="h-4 w-4 mr-1" /> 数据签名验证
            </Button>
            <Button size="sm">报表配置</Button>
          </>
        }
      />

      {/* 查询条件 */}
      <div className="px-6 py-4 border-b bg-muted/30 grid grid-cols-6 gap-3 items-end">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">报表类型 *</label>
          <Select defaultValue="purchase_monthly">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="purchase_monthly">采购入库月报（GSP §83）</SelectItem>
              <SelectItem value="sales_monthly">销售出库月报（GSP §85）</SelectItem>
              <SelectItem value="inventory_monthly">库存盘点月报（GSP §95）</SelectItem>
              <SelectItem value="cold_monthly">冷链温度月报（GSP §64）</SelectItem>
              <SelectItem value="expiry_monthly">近效期/不合格月报（GSP §50）</SelectItem>
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
          <label className="text-xs text-muted-foreground mb-1 block">货主</label>
          <Select defaultValue="all">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="o1">货主 A</SelectItem>
              <SelectItem value="o2">货主 B</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">仓库</label>
          <Select defaultValue="W001">
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="W001">W001 北京天竺仓</SelectItem>
              <SelectItem value="all">全部</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search className="h-3.5 w-3.5 mr-1" /> 查询</Button>
        </div>
      </div>

      {/* 结果摘要 KPI */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">入库单数</div>
          <div className="text-xl font-bold mt-1">42</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">入库总件数</div>
          <div className="text-xl font-bold mt-1">2,486 <span className="text-xs font-normal text-muted-foreground">件</span></div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">入库总金额</div>
          <div className="text-xl font-bold mt-1">¥523K</div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning">部分接收</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">3</div>
        </Card>
        <Card className="p-3 border-destructive/40 bg-destructive/5">
          <div className="text-xs text-destructive">拒收</div>
          <div className="text-xl font-bold mt-1 text-destructive">1</div>
        </Card>
      </div>

      {/* 主区：明细表 + 右侧操作 */}
      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">采购入库明细（2026-04）</div>
            <span className="text-xs text-muted-foreground font-mono">查询用时 0.42s · MD5: a3f2…b9x7</span>
          </div>
          <DataTable columns={cols} data={PURCHASE_DATA} rowKey={(r) => r.asn} />
          <div className="mt-3 text-xs text-muted-foreground flex items-center justify-between">
            <span>共 6 条 · 显示前 6 行（完整 42 行通过导出查看）</span>
            <div className="flex gap-1.5">
              <Button variant="outline" size="sm" disabled>上一页</Button>
              <Button variant="outline" size="sm">下一页</Button>
            </div>
          </div>
        </div>

        {/* 右侧：导出 + 审计 + 合规 */}
        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download className="h-4 w-4" /> 导出报表
            </div>
            <div className="space-y-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText className="h-4 w-4 mr-2" /> 导出 PDF（含签字位）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet className="h-4 w-4 mr-2" /> 导出 Excel（明细）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <Database className="h-4 w-4 mr-2" /> 导出 JSON（监管报送）
              </Button>
            </div>
            <div className="mt-3 pt-3 border-t text-xs text-muted-foreground space-y-1">
              <div className="flex justify-between"><span>含明细</span><span>✓</span></div>
              <div className="flex justify-between"><span>含 MD5 签名</span><span>✓</span></div>
              <div className="flex justify-between"><span>双方盖章位</span><span>仅 PDF</span></div>
            </div>
          </Card>

          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield className="h-4 w-4" /> GSP 合规
            </div>
            <ul className="text-xs text-muted-foreground space-y-1">
              <li>· 查询/导出写 H2 审计</li>
              <li>· MD5 签名防篡改</li>
              <li>· 留存 5 年（§95）</li>
              <li>· 监管 EDI 通过 ERP</li>
            </ul>
          </Card>

          <Card className="p-4">
            <div className="text-sm font-semibold mb-3">查询/导出审计</div>
            <AuditTimeline
              events={AUDIT_EVENTS}
              expandedId={expanded}
              onExpand={(id) => setExpanded(id === expanded ? undefined : id)}
            />
          </Card>
        </div>
      </div>
    </div>
  );
}
