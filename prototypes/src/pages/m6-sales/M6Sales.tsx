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

interface SalesRow {
  date: string;
  so: string;
  customer: string;
  itemName: string;
  qty: number;
  amount: string;
  reviewer: string;
  status: "shipped" | "returned";
}

const SALES_DATA: SalesRow[] = [
  { date: "2026-04-03", so: "SO-2026-0011", customer: "同仁堂朝阳店", itemName: "葡萄糖注射液 500ml × 24",
    qty: 120, amount: "11,760.00", reviewer: "李四 (u002)", status: "shipped" },
  { date: "2026-04-08", so: "SO-2026-0019", customer: "国大药房海淀店", itemName: "重组人胰岛素 3ml × 5",
    qty: 30, amount: "14,580.00", reviewer: "张三 (u001)", status: "shipped" },
  { date: "2026-04-15", so: "SO-2026-0028", customer: "益丰大药房通州店", itemName: "复方甘草口服溶液",
    qty: 60, amount: "2,160.00", reviewer: "王五 (u003)", status: "shipped" },
  { date: "2026-04-19", so: "SO-2026-0035", customer: "全药网泰康店", itemName: "甘精胰岛素 3ml × 5",
    qty: 25, amount: "15,750.00", reviewer: "李四 (u002)", status: "shipped" },
  { date: "2026-04-22", so: "SO-2026-0033", customer: "同仁堂朝阳店", itemName: "盐酸吗啡片 10mg × 100",
    qty: 12, amount: "3,840.00", reviewer: "张三 (u001)", status: "returned" },
];

const STATUS_LABEL = {
  shipped: { text: "已发货", color: "bg-wms-success/10 text-wms-success" },
  returned: { text: "退货", color: "bg-wms-warning/10 text-wms-warning" },
};

/**
 * M6Sales — M6-002b 销售出库月报
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-002（GSP §85 销售出库月报 / 查询 + 多格式导出 / 一货一单留底）
 * Wave：Wave 3.0（M6 报表合规）
 * 业务约束：每条出库对应一张随货同行单（M4-005）；退货独立标记；保留 5 年
 *
 * @example
 *   <M6Sales />
 */
export function M6Sales() {
  const cols: DataTableColumn<SalesRow>[] = [
    { key: "date", header: "日期", render: (r) => <span className="font-mono text-xs">{r.date}</span> },
    { key: "so", header: "SO 号", render: (r) => <span className="font-mono text-xs text-primary">{r.so}</span> },
    { key: "customer", header: "客户", render: (r) => <span className="text-sm">{r.customer}</span> },
    { key: "item", header: "商品", render: (r) => <span className="text-sm">{r.itemName}</span> },
    { key: "qty", header: "数量", align: "right", render: (r) => <span className="text-sm font-medium">{r.qty}</span> },
    { key: "amount", header: "金额", align: "right", render: (r) => <span className="text-sm font-mono">¥{r.amount}</span> },
    { key: "reviewer", header: "复核员", render: (r) => <span className="text-xs">{r.reviewer}</span> },
    { key: "status", header: "状态", render: (r) => {
      const m = STATUS_LABEL[r.status];
      return <span className={`text-xs px-1.5 py-0.5 rounded ${m.color}`}>{m.text}</span>;
    }},
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="销售出库月报"
        subtitle="M6-002b · GSP §85 · 月度查询 + 多格式导出 · 一货一单留底"
        actions={
          <Button variant="outline" size="sm">
            <Database className="h-4 w-4 mr-1" /> 数据签名验证
          </Button>
        }
      />

      <div className="px-6 py-4 border-b bg-muted/30 grid grid-cols-5 gap-3 items-end">
        <div><label className="text-xs text-muted-foreground mb-1 block">起始日期</label>
          <Input type="date" defaultValue="2026-04-01" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">截止日期</label>
          <Input type="date" defaultValue="2026-04-30" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">客户</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="c1">同仁堂朝阳店</SelectItem>
            <SelectItem value="c2">国大药房海淀店</SelectItem>
          </SelectContent></Select></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">含退货</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="shipped">仅已发货</SelectItem>
            <SelectItem value="returned">仅退货</SelectItem>
          </SelectContent></Select></div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search className="h-3.5 w-3.5 mr-1" /> 查询</Button>
        </div>
      </div>

      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">出库单数</div>
          <div className="text-xl font-bold mt-1">38</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">出库总件数</div>
          <div className="text-xl font-bold mt-1">2,860 <span className="text-xs font-normal text-muted-foreground">件</span></div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">出库总金额</div>
          <div className="text-xl font-bold mt-1">¥486K</div>
        </Card>
        <Card className="p-3 border-wms-warning/40 bg-wms-warning/5">
          <div className="text-xs text-wms-warning">退货</div>
          <div className="text-xl font-bold mt-1 text-wms-warning">2</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">随货同行单覆盖</div>
          <div className="text-xl font-bold mt-1 text-wms-success">100%</div>
        </Card>
      </div>

      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold">销售出库明细（2026-04）</div>
            <span className="text-xs text-muted-foreground font-mono">共 38 条 · MD5: c5d4…f2a8</span>
          </div>
          <DataTable columns={cols} data={SALES_DATA} rowKey={(r) => r.so} />
          <div className="mt-3 text-xs text-muted-foreground">显示 5 / 38 条 · 完整明细通过导出查看</div>
        </div>

        <div className="space-y-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download className="h-4 w-4" /> 导出
            </div>
            <div className="space-y-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText className="h-4 w-4 mr-2" /> 导出 PDF
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet className="h-4 w-4 mr-2" /> 导出 Excel
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <Database className="h-4 w-4 mr-2" /> 导出 JSON（监管报送）
              </Button>
            </div>
          </Card>
          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield className="h-4 w-4" /> GSP §85 合规
            </div>
            <ul className="text-xs text-muted-foreground space-y-1">
              <li>· 出库时同步生成随货同行单</li>
              <li>· 含批号 / 效期 / 生产企业</li>
              <li>· 留存 5 年</li>
              <li>· 监管 EDI 通过 ERP</li>
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
