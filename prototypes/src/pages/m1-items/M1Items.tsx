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
import { Plus, Search, Download, Upload, FileText } from "lucide-react";

interface Item {
  id: string;
  code: string;
  name: string;
  spec: string;
  manufacturer: string;
  category: "rx" | "otc" | "controlled" | "narcotic" | "psychotropic";
  storage: "RT" | "CR" | "CL" | "FR";
  udi: string;
  batchTracked: boolean;
  qualDocsValid: boolean;
  status: "active" | "isolated" | "discontinued";
}

const MOCK: Item[] = [
  {
    id: "i1", code: "P-001234", name: "葡萄糖注射液", spec: "500ml × 24",
    manufacturer: "山东齐鲁制药", category: "rx", storage: "RT",
    udi: "(01)06901234567890(17)280301(10)20260301A",
    batchTracked: true, qualDocsValid: true, status: "active",
  },
  {
    id: "i2", code: "P-001235", name: "重组人胰岛素注射液", spec: "3ml:300IU × 5",
    manufacturer: "甘李药业", category: "rx", storage: "CR",
    udi: "(01)06901234567891(17)270615(10)20260315B",
    batchTracked: true, qualDocsValid: true, status: "active",
  },
  {
    id: "i3", code: "P-002001", name: "盐酸吗啡片", spec: "10mg × 100",
    manufacturer: "东北制药", category: "narcotic", storage: "RT",
    udi: "(01)06901234567892(17)270901(10)20260101N",
    batchTracked: true, qualDocsValid: true, status: "active",
  },
  {
    id: "i4", code: "P-003045", name: "复方甘草口服溶液", spec: "100ml × 1",
    manufacturer: "广州白云山", category: "otc", storage: "RT",
    udi: "(01)06901234567893(17)281201(10)20261105",
    batchTracked: false, qualDocsValid: false, status: "isolated",
  },
  {
    id: "i5", code: "P-004088", name: "辉瑞 mRNA 疫苗（示例）", spec: "0.3ml × 6",
    manufacturer: "Pfizer Inc.", category: "rx", storage: "FR",
    udi: "(01)06901234567894(17)270301(10)20260201F",
    batchTracked: true, qualDocsValid: true, status: "active",
  },
];

const CATEGORY_LABEL = {
  rx: { text: "处方药", color: "bg-primary/10 text-primary" },
  otc: { text: "OTC", color: "bg-muted text-muted-foreground" },
  controlled: { text: "管制药", color: "bg-destructive/10 text-destructive" },
  narcotic: { text: "麻醉药", color: "bg-destructive/10 text-destructive font-semibold" },
  psychotropic: { text: "精神药", color: "bg-wms-warning/10 text-wms-warning" },
};

const STORAGE_LABEL = {
  RT: { text: "常温", color: "bg-muted text-muted-foreground" },
  CR: { text: "冷藏 2-8℃", color: "bg-wms-cold/10 text-wms-cold" },
  CL: { text: "阴凉 ≤20℃", color: "bg-wms-cold/10 text-wms-cold/80" },
  FR: { text: "冷冻 -25℃", color: "bg-wms-cold/20 text-wms-cold font-medium" },
};

/**
 * M1Items — M1-001 商品档案管理
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M1-001（商品档案 + UDI + 特管 + 资质 + 多批号）
 * Wave：Wave 2.0（M1 基础数据）
 * 业务约束：UDI 唯一；管制类（麻醉/精神）必须双签字；冷链商品强制温区匹配
 *
 * @example
 *   <M1Items />
 */
export function M1Items() {
  const cols: DataTableColumn<Item>[] = [
    {
      key: "code",
      header: "商品编码",
      render: (r) => <span className="font-mono text-xs text-primary">{r.code}</span>,
    },
    {
      key: "name",
      header: "品名 / 规格",
      render: (r) => (
        <div>
          <div className="font-medium text-sm">{r.name}</div>
          <div className="text-xs text-muted-foreground mt-0.5">{r.spec}</div>
        </div>
      ),
    },
    {
      key: "manufacturer",
      header: "生产企业",
      render: (r) => <span className="text-sm">{r.manufacturer}</span>,
    },
    {
      key: "category",
      header: "分类",
      render: (r) => {
        const meta = CATEGORY_LABEL[r.category];
        return (
          <span className={`text-xs px-1.5 py-0.5 rounded ${meta.color}`}>{meta.text}</span>
        );
      },
    },
    {
      key: "storage",
      header: "储存",
      render: (r) => {
        const meta = STORAGE_LABEL[r.storage];
        return (
          <span className={`text-xs px-1.5 py-0.5 rounded ${meta.color}`}>{meta.text}</span>
        );
      },
    },
    {
      key: "udi",
      header: "UDI",
      render: (r) => (
        <span className="font-mono text-[11px] text-muted-foreground">{r.udi.slice(0, 24)}…</span>
      ),
    },
    {
      key: "qualDocs",
      header: "资质",
      render: (r) =>
        r.qualDocsValid ? (
          <span className="text-xs text-wms-success flex items-center gap-1">
            <FileText data-icon="inline-start" /> 完整
          </span>
        ) : (
          <span className="text-xs text-destructive flex items-center gap-1">
            <FileText data-icon="inline-start" /> 缺失
          </span>
        ),
    },
    {
      key: "status",
      header: "状态",
      render: (r) =>
        r.status === "active" ? (
          <StatusBadge status="qualified" size="sm" label="正常" />
        ) : r.status === "isolated" ? (
          <StatusBadge status="isolated" size="sm" label="冻结" />
        ) : (
          <StatusBadge status="expired" size="sm" label="停用" />
        ),
    },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="商品档案管理"
        subtitle="M1-001 · 含 UDI / 资质 / 特管标签 / 多批号配置 · GSP 合规"
        actions={
          <>
            <Button size="sm" variant="ghost" onClick={() => (window.location.hash = "#m6-custom")} title="另存为自定义报表">
              ⇲ 另存为报表
            </Button>
            <Button variant="outline" size="sm">
              <Upload data-icon="inline-start" /> 导入 Excel
            </Button>
            <Button variant="outline" size="sm">
              <Download data-icon="inline-start" /> 导出
            </Button>
            <Button size="sm">
              <Plus data-icon="inline-start" /> 新增商品
            </Button>
          </>
        }
      />

      {/* 筛选栏 */}
      <div className="px-6 py-4 border-b bg-muted/30 grid grid-cols-5 gap-3 items-end">
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">关键字搜索</label>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input className="pl-9" placeholder="编码 / 品名 / 生产企业" />
          </div>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">分类</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="rx">处方药</SelectItem>
              <SelectItem value="otc">OTC</SelectItem>
              <SelectItem value="narcotic">麻醉药</SelectItem>
              <SelectItem value="psychotropic">精神药</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">储存条件</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="RT">常温</SelectItem>
              <SelectItem value="CR">冷藏</SelectItem>
              <SelectItem value="CL">阴凉</SelectItem>
              <SelectItem value="FR">冷冻</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div>
          <label className="text-xs text-muted-foreground mb-1 block">资质</label>
          <Select>
            <SelectTrigger>
              <SelectValue placeholder="全部" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">全部</SelectItem>
              <SelectItem value="valid">完整</SelectItem>
              <SelectItem value="missing">缺失</SelectItem>
              <SelectItem value="expiring">即将过期</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm">查询</Button>
        </div>
      </div>

      {/* 数据统计 */}
      <div className="px-6 py-3 border-b flex items-center gap-6 text-xs text-muted-foreground">
        <span>共 <span className="font-semibold text-foreground">2,348</span> 条</span>
        <span>处方药 <span className="font-semibold text-foreground">1,820</span></span>
        <span>麻醉/精神 <span className="font-semibold text-destructive">42</span></span>
        <span>冷链 <span className="font-semibold text-wms-cold">186</span></span>
        <span>资质即将过期 <span className="font-semibold text-wms-warning">8</span></span>
      </div>

      <DataTable columns={cols} data={MOCK} rowKey={(r) => r.id} />

      <Card className="mx-6 my-4 p-3 bg-muted/30">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">
            每页 50 条 · 共 2,348 条 · 第 1/47 页
          </span>
          <div className="flex gap-1.5">
            <Button variant="outline" size="sm" disabled>上一页</Button>
            <Button variant="outline" size="sm">下一页</Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
