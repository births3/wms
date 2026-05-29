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
import { useState } from "react";
import {
  PageHeader,
  DataTable,
  StatusBadge,
  type DataTableColumn,
} from "@wms/ui";
import { Search, Download, FileText, FileSpreadsheet, Database, Shield, AlertOctagon } from "lucide-react";

interface SpecialDrugRow {
  date: string;
  itemCode: string;
  itemName: string;
  spec: string;
  batch: string;
  inQty: number;
  outQty: number;
  balanceQty: number;
  signer1: string;
  signer2: string;
  doc: string;
}

const NARCOTIC_DATA: SpecialDrugRow[] = [
  { date: "2026-04-02", itemCode: "P-002001", itemName: "盐酸吗啡片", spec: "10mg × 100",
    batch: "20260101N", inQty: 24, outQty: 0, balanceQty: 24, signer1: "张三 (u001)", signer2: "李四 (u002)",
    doc: "PO-2026-0001" },
  { date: "2026-04-08", itemCode: "P-002001", itemName: "盐酸吗啡片", spec: "10mg × 100",
    batch: "20260101N", inQty: 0, outQty: 4, balanceQty: 20, signer1: "李四 (u002)", signer2: "王五 (u003)",
    doc: "SO-2026-0019" },
  { date: "2026-04-15", itemCode: "P-002005", itemName: "盐酸哌替啶注射液", spec: "50mg × 10",
    batch: "20260201N", inQty: 12, outQty: 0, balanceQty: 12, signer1: "张三 (u001)", signer2: "赵六 (u004)",
    doc: "PO-2026-0027" },
  { date: "2026-04-22", itemCode: "P-002001", itemName: "盐酸吗啡片", spec: "10mg × 100",
    batch: "20260101N", inQty: 0, outQty: 2, balanceQty: 18, signer1: "王五 (u003)", signer2: "李四 (u002)",
    doc: "SO-2026-0035" },
  { date: "2026-04-30", itemCode: "P-002005", itemName: "盐酸哌替啶注射液", spec: "50mg × 10",
    batch: "20260201N", inQty: 0, outQty: 1, balanceQty: 11, signer1: "张三 (u001)", signer2: "赵六 (u004)",
    doc: "SO-2026-0042" },
];

type DrugCategory = "narcotic" | "psychotropic" | "radioactive" | "blood";

const CATEGORY_META: Record<DrugCategory, { label: string; color: string; clause: string; desc: string }> = {
  narcotic:     { label: "麻醉药品", color: "border-destructive bg-destructive/5", clause: "GSP §82 + 麻管法",
                  desc: "双人验收 + 双人发货 + 月度盘点 + 红票" },
  psychotropic: { label: "精神药品（一类）", color: "border-destructive/70 bg-destructive/5", clause: "GSP §82 + 精管法",
                  desc: "双人验收 + 双人发货 + 月度盘点" },
  radioactive:  { label: "放射性药品", color: "border-wms-warning bg-wms-warning/5", clause: "GSP §82 + 放管条例",
                  desc: "专人专管 + 防辐射隔离 + 月度上报" },
  blood:        { label: "血液制品", color: "border-wms-cold/40 bg-wms-cold/5", clause: "GSP §82 + 血液制品管理",
                  desc: "冷链全程 + 来源凭证 + 一货一档" },
};

/**
 * M6Special — M6-004 特殊管理药品专用台账
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-004（麻精/放射/血液制品专用格式 / 双人签字 / 监管报送）
 * Wave：Wave 3.0
 * 业务约束：双人签字必填；月底必须盘点；走 ERP 上报国家药监；红色台账留 5 年
 *
 * @example
 *   <M6Special />
 */
export function M6Special() {
  const [category, setCategory] = useState<DrugCategory>("narcotic");
  const meta = CATEGORY_META[category];

  const cols: DataTableColumn<SpecialDrugRow>[] = [
    { key: "date", header: "日期", render: (r) => <span className="font-mono text-xs">{r.date}</span> },
    { key: "code", header: "商品",
      render: (r) => <div>
        <div className="font-mono text-xs text-primary">{r.itemCode}</div>
        <div className="text-sm">{r.itemName}</div>
        <div className="text-xs text-muted-foreground">{r.spec}</div>
      </div>
    },
    { key: "batch", header: "批号", render: (r) => <span className="font-mono text-xs">{r.batch}</span> },
    { key: "in", header: "入库", align: "right", render: (r) =>
      r.inQty > 0 ? <span className="text-sm font-medium text-wms-success">+{r.inQty}</span> : <span className="text-muted-foreground">—</span>
    },
    { key: "out", header: "出库", align: "right", render: (r) =>
      r.outQty > 0 ? <span className="text-sm font-medium text-destructive">-{r.outQty}</span> : <span className="text-muted-foreground">—</span>
    },
    { key: "balance", header: "结存", align: "right", render: (r) =>
      <span className="text-sm font-bold">{r.balanceQty}</span>
    },
    { key: "signers", header: "双人签字", render: (r) =>
      <div className="text-xs">
        <div>{r.signer1}</div>
        <div className="text-muted-foreground">+ {r.signer2}</div>
      </div>
    },
    { key: "doc", header: "凭证", render: (r) => <span className="font-mono text-xs">{r.doc}</span> },
  ];

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="特殊管理药品专用台账"
        subtitle="M6-004 · 麻精 / 放射 / 血液制品 · 双人签字 + 月度盘点 + 监管报送"
        actions={
          <>
            <Button variant="outline" size="sm">
              <AlertOctagon data-icon="inline-start" /> 缺签预警
            </Button>
            <Button size="sm">
              <Database data-icon="inline-start" /> 上报 ERP
            </Button>
          </>
        }
      />

      {/* 类别切换 */}
      <div className="px-6 py-3 border-b flex items-center gap-2 flex-wrap">
        {(Object.keys(CATEGORY_META) as DrugCategory[]).map((c) => (
          <Button
            key={c}
            variant={category === c ? "default" : "outline"}
            size="sm"
            onClick={() => setCategory(c)}
          >
            {CATEGORY_META[c].label}
          </Button>
        ))}
        <span className="ml-auto text-xs text-muted-foreground">
          当前：<span className={`px-1.5 py-0.5 border rounded ${meta.color}`}>{meta.label}</span>
          <span className="ml-2 font-mono">{meta.clause}</span>
        </span>
      </div>

      {/* 法规说明 */}
      <div className={`mx-6 mt-4 p-3 border rounded-md flex items-start gap-2 ${meta.color}`}>
        <AlertOctagon className="size-4 flex-shrink-0 mt-0.5 text-destructive" />
        <div className="text-xs flex-1">
          <span className="font-medium">{meta.label} · {meta.clause}</span>
          <span className="text-muted-foreground ml-2">{meta.desc}</span>
        </div>
      </div>

      {/* 筛选 */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3 items-end">
        <div><label className="text-xs text-muted-foreground mb-1 block">起始</label>
          <Input type="date" defaultValue="2026-04-01" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">截止</label>
          <Input type="date" defaultValue="2026-04-30" /></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">商品</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="P-002001">盐酸吗啡片</SelectItem>
            <SelectItem value="P-002005">盐酸哌替啶</SelectItem>
          </SelectContent></Select></div>
        <div><label className="text-xs text-muted-foreground mb-1 block">操作类型</label>
          <Select defaultValue="all"><SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="all">全部</SelectItem>
            <SelectItem value="in">仅入库</SelectItem>
            <SelectItem value="out">仅出库</SelectItem>
          </SelectContent></Select></div>
        <div className="flex gap-2">
          <Button variant="outline" size="sm">重置</Button>
          <Button size="sm"><Search data-icon="inline-start" /> 查询</Button>
        </div>
      </div>

      {/* KPI */}
      <div className="px-6 py-4 border-b grid grid-cols-5 gap-3">
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">月初结存</div>
          <div className="text-xl font-bold mt-1">8</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">本月入库</div>
          <div className="text-xl font-bold mt-1 text-wms-success">+36</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">本月出库</div>
          <div className="text-xl font-bold mt-1 text-destructive">-7</div>
        </Card>
        <Card className="p-3">
          <div className="text-xs text-muted-foreground">月末结存</div>
          <div className="text-xl font-bold mt-1">37</div>
        </Card>
        <Card className="p-3 border-wms-success/40 bg-wms-success/5">
          <div className="text-xs text-wms-success">双签覆盖率</div>
          <div className="text-xl font-bold mt-1 text-wms-success">100%</div>
          <div className="text-[11px] text-muted-foreground">5/5 笔</div>
        </Card>
      </div>

      {/* 明细 */}
      <div className="px-6 py-4 grid grid-cols-[1fr_300px] gap-4">
        <div>
          <div className="flex items-center justify-between mb-3">
            <div className="text-sm font-semibold flex items-center gap-2">
              <StatusBadge status="qualified" size="sm" label="本月已盘" />
              <span>{meta.label}台账明细（2026-04）</span>
            </div>
            <span className="text-xs text-muted-foreground font-mono">MD5: 7a3f…b8e2</span>
          </div>
          <DataTable columns={cols} data={NARCOTIC_DATA} rowKey={(r) => `${r.date}-${r.batch}-${r.doc}`} />
          <div className="mt-3 text-xs text-muted-foreground">
            5 笔记录全部含双人签字 ✓ · 月底已盘点 · 已上报 ERP（药监 EDI）
          </div>
        </div>
        <div className="flex flex-col gap-4">
          <Card className="p-4">
            <div className="text-sm font-semibold mb-3 flex items-center gap-2">
              <Download data-icon="inline-start" /> 监管报送
            </div>
            <div className="flex flex-col gap-2">
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileText data-icon="inline-start" /> PDF（红色台账）
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <FileSpreadsheet data-icon="inline-start" /> Excel
              </Button>
              <Button variant="outline" size="sm" className="w-full justify-start">
                <Database data-icon="inline-start" /> JSON（药监 EDI）
              </Button>
            </div>
          </Card>
          <Card className="p-4 bg-muted/30">
            <div className="text-sm font-semibold mb-2 flex items-center gap-2">
              <Shield data-icon="inline-start" /> 合规要点
            </div>
            <ul className="text-xs text-muted-foreground flex flex-col gap-1">
              <li>· 双人签字强制（u001 + u002）</li>
              <li>· 月底必须盘点（5/30 已完成）</li>
              <li>· 上报 ERP → 国家药监</li>
              <li>· 红色台账留 5 年</li>
              <li>· 失窃/损耗 24h 内报警</li>
            </ul>
          </Card>
        </div>
      </div>
    </div>
  );
}
