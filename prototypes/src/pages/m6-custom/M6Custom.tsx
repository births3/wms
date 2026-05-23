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
  type DataTableColumn,
} from "@wms/ui";
import {
  Save, Star, BarChart3, LineChart, PieChart, AreaChart, Table as TableIcon,
  Download, Trash2, ExternalLink, GripVertical, X, Plus, Database,
} from "lucide-react";

/**
 * M6Custom — M6-003 业务报表（自建查询 / 行列值三栏）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-003（自定义维度+指标+图表+保存模板+订阅+Metabase 嵌入）
 * Wave：Wave 0.5（演示原型）→ Wave 5（Metabase 嵌入正式上线）
 * 业务约束：仿 Metabase 拖拽体验；不替代 GSP 法定报表（M6-001）；查询写 H2 审计
 *
 * 参考 ADR-0023：混合方案 — 当前 mock；Wave 5 接 Metabase iframe
 *
 * @example
 *   <M6Custom />
 */

interface PreviewRow {
  dim: string;
  m1: number;
  m2: number;
  m3: number;
}

const PREVIEW_DATA: PreviewRow[] = [
  { dim: "国药控股北京", m1: 12, m2: 1820, m3: 215400 },
  { dim: "上海医药华东", m1: 9, m2: 1320, m3: 168200 },
  { dim: "九州通医药",   m1: 8, m2: 980,  m3: 92400 },
  { dim: "甘李药业",     m1: 5, m2: 580,  m3: 286800 },
  { dim: "东北制药",     m1: 4, m2: 480,  m3: 24600 },
  { dim: "其他（12 家）", m1: 18, m2: 2240, m3: 137200 },
];

const SAVED_TEMPLATES = [
  { id: "t1", name: "供应商月度采购排行", scope: "私有", lastRun: "今日 09:14", isFavorite: true, dashboard: 12 },
  { id: "t2", name: "冷链商品销售趋势", scope: "私有", lastRun: "昨日 16:30", isFavorite: true, dashboard: 13 },
  { id: "t3", name: "麻精药品月度核对", scope: "部门", lastRun: "5/15 10:08", dashboard: 14 },
  { id: "t4", name: "客户退货率分析", scope: "全局", lastRun: "5/12 14:22", dashboard: 15 },
  { id: "t5", name: "盘点差异趋势", scope: "私有", lastRun: "5/10 11:00", dashboard: 16 },
];

interface FieldChip {
  id: string;
  label: string;
  type: "dim" | "metric";
  agg?: "sum" | "avg" | "count";
}

const ALL_FIELDS: FieldChip[] = [
  { id: "supplier", label: "供应商", type: "dim" },
  { id: "date_month", label: "日期（月）", type: "dim" },
  { id: "date_day", label: "日期（日）", type: "dim" },
  { id: "warehouse", label: "仓库", type: "dim" },
  { id: "owner", label: "货主", type: "dim" },
  { id: "category", label: "商品分类", type: "dim" },
  { id: "asn_count", label: "入库单数", type: "metric", agg: "sum" },
  { id: "qty", label: "总件数", type: "metric", agg: "sum" },
  { id: "amount", label: "总金额", type: "metric", agg: "sum" },
  { id: "exception", label: "异常单数", type: "metric", agg: "count" },
  { id: "avg_price", label: "平均单价", type: "metric", agg: "avg" },
];

interface FilterCondition {
  id: string;
  field: string;
  op: string;
  value: string;
}
interface FilterGroup {
  id: string;
  connector: "AND" | "OR";
  conditions: FilterCondition[];
}

const INITIAL_FILTERS: FilterGroup[] = [
  {
    id: "g1",
    connector: "AND",
    conditions: [
      { id: "c1", field: "date_month", op: ">=", value: "2026-04" },
      { id: "c2", field: "date_month", op: "<=", value: "2026-04" },
      { id: "c3", field: "amount", op: ">", value: "10000" },
    ],
  },
];

type ChartType = "table" | "bar" | "line" | "pie" | "area";

export function M6Custom() {
  const [chartType, setChartType] = useState<ChartType>("table");
  const [reportName, setReportName] = useState("供应商月度采购排行（草稿）");

  const cols: DataTableColumn<PreviewRow>[] = [
    { key: "dim", header: "供应商（行）", render: (r) => <span className="text-sm font-medium">{r.dim}</span> },
    { key: "m1", header: "Σ 入库单数", align: "right", render: (r) => <span className="text-sm">{r.m1}</span> },
    { key: "m2", header: "Σ 总件数", align: "right", render: (r) => <span className="text-sm">{r.m2.toLocaleString()}</span> },
    { key: "m3", header: "Σ 总金额", align: "right", render: (r) => <span className="text-sm font-mono">¥{r.m3.toLocaleString()}</span> },
  ];

  const maxAmount = Math.max(...PREVIEW_DATA.map((r) => r.m3));

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="业务报表（自建）"
        subtitle="M6-003 · 拖拽建报表 · 当前演示原型 · Wave 5 接 Metabase（参 ADR-0023）"
        actions={
          <>
            <Button variant="outline" size="sm">
              <ExternalLink className="h-4 w-4 mr-1" /> 在 Metabase 中打开
            </Button>
            <Button variant="outline" size="sm">
              <Download className="h-4 w-4 mr-1" /> 导出
            </Button>
            <Button size="sm">
              <Save className="h-4 w-4 mr-1" /> 保存模板
            </Button>
          </>
        }
      />

      {/* 报表元信息 */}
      <div className="px-6 py-3 border-b flex items-center gap-3">
        <Input
          value={reportName}
          onChange={(e) => setReportName(e.target.value)}
          className="max-w-[300px] h-8 font-medium"
        />
        <span className="text-xs text-muted-foreground">·</span>
        <Select defaultValue="purchase">
          <SelectTrigger className="w-[180px] h-8 text-xs">
            <span className="text-muted-foreground mr-1">数据源：</span><SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="purchase">采购入库</SelectItem>
            <SelectItem value="sales">销售出库</SelectItem>
            <SelectItem value="inventory">库存</SelectItem>
            <SelectItem value="cold">冷链温度</SelectItem>
            <SelectItem value="quality">质量管理</SelectItem>
          </SelectContent>
        </Select>
        <Select defaultValue="private">
          <SelectTrigger className="w-[120px] h-8 text-xs">
            <span className="text-muted-foreground mr-1">权限：</span><SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="private">私有</SelectItem>
            <SelectItem value="dept">部门共享</SelectItem>
            <SelectItem value="global">全局</SelectItem>
          </SelectContent>
        </Select>
        <span className="ml-auto text-xs text-muted-foreground">最后保存 5 分钟前 · 草稿未发布</span>
      </div>

      <div className="px-6 py-4 grid grid-cols-[280px_1fr] gap-4">
        {/* 左侧：字段抽屉（拖入源）*/}
        <div className="space-y-3">
          <Card className="p-3">
            <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1">
              <Database className="h-3 w-3" /> 可用字段（拖到右侧）
            </div>
            <div className="text-[11px] text-muted-foreground mb-2 italic">维度</div>
            <div className="space-y-1 mb-3">
              {ALL_FIELDS.filter((f) => f.type === "dim").map((f) => (
                <div key={f.id} className="flex items-center gap-2 px-2 py-1.5 bg-muted/30 rounded text-xs cursor-move hover:bg-primary/10">
                  <GripVertical className="h-3 w-3 text-muted-foreground flex-shrink-0" />
                  <span className="flex-1">{f.label}</span>
                  <span className="text-[10px] px-1 py-0.5 bg-primary/10 text-primary rounded">D</span>
                </div>
              ))}
            </div>
            <div className="text-[11px] text-muted-foreground mb-2 italic">指标</div>
            <div className="space-y-1">
              {ALL_FIELDS.filter((f) => f.type === "metric").map((f) => (
                <div key={f.id} className="flex items-center gap-2 px-2 py-1.5 bg-muted/30 rounded text-xs cursor-move hover:bg-wms-success/10">
                  <GripVertical className="h-3 w-3 text-muted-foreground flex-shrink-0" />
                  <span className="flex-1">{f.label}</span>
                  <span className="text-[10px] px-1 py-0.5 bg-wms-success/10 text-wms-success rounded">M</span>
                </div>
              ))}
            </div>
          </Card>
        </div>

        {/* 右侧：行/列/值 三栏 + 过滤 + 预览 */}
        <div className="space-y-3">
          {/* 三栏配置 */}
          <div className="grid grid-cols-3 gap-3">
            <Card className="p-3">
              <div className="text-xs font-semibold mb-2">行（分组）</div>
              <div className="space-y-1.5 min-h-[80px] bg-muted/20 p-2 rounded border-2 border-dashed">
                <div className="flex items-center gap-2 px-2 py-1.5 bg-primary/10 text-primary rounded text-xs">
                  <GripVertical className="h-3 w-3" />
                  <span className="flex-1 font-medium">供应商</span>
                  <X className="h-3 w-3 cursor-pointer" />
                </div>
              </div>
            </Card>
            <Card className="p-3">
              <div className="text-xs font-semibold mb-2">列（透视）</div>
              <div className="space-y-1.5 min-h-[80px] bg-muted/20 p-2 rounded border-2 border-dashed">
                <div className="text-[11px] text-muted-foreground italic px-2">拖入维度展开为列</div>
              </div>
            </Card>
            <Card className="p-3">
              <div className="text-xs font-semibold mb-2">值（指标聚合）</div>
              <div className="space-y-1.5 min-h-[80px] bg-muted/20 p-2 rounded border-2 border-dashed">
                {[
                  { l: "Σ 入库单数", agg: "sum" },
                  { l: "Σ 总件数", agg: "sum" },
                  { l: "Σ 总金额", agg: "sum" },
                ].map((m, i) => (
                  <div key={i} className="flex items-center gap-2 px-2 py-1.5 bg-wms-success/10 text-wms-success rounded text-xs">
                    <GripVertical className="h-3 w-3" />
                    <span className="flex-1 font-medium">{m.l}</span>
                    <X className="h-3 w-3 cursor-pointer" />
                  </div>
                ))}
              </div>
            </Card>
          </div>

          {/* 过滤条件（AND/OR 组合）*/}
          <Card className="p-3">
            <div className="flex items-center justify-between mb-2">
              <div className="text-xs font-semibold">过滤条件</div>
              <Button variant="ghost" size="sm" className="h-6 text-xs"><Plus className="h-3 w-3 mr-0.5" /> 加条件组</Button>
            </div>
            {INITIAL_FILTERS.map((g) => (
              <div key={g.id} className="border rounded p-2 bg-muted/20">
                <div className="flex items-center gap-2 mb-1.5">
                  <Select defaultValue={g.connector}>
                    <SelectTrigger className="h-7 w-[80px] text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="AND">AND</SelectItem>
                      <SelectItem value="OR">OR</SelectItem>
                    </SelectContent>
                  </Select>
                  <span className="text-xs text-muted-foreground">同时满足以下 {g.conditions.length} 个条件</span>
                </div>
                <div className="space-y-1.5">
                  {g.conditions.map((c) => {
                    const f = ALL_FIELDS.find((af) => af.id === c.field);
                    return (
                      <div key={c.id} className="flex items-center gap-2 text-xs">
                        <Select defaultValue={c.field}>
                          <SelectTrigger className="h-7 w-[140px] text-xs"><SelectValue /></SelectTrigger>
                          <SelectContent>
                            {ALL_FIELDS.filter((af) => af.type === "dim" || af.type === "metric").map((af) => (
                              <SelectItem key={af.id} value={af.id}>{af.label}</SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                        <Select defaultValue={c.op}>
                          <SelectTrigger className="h-7 w-[80px] text-xs"><SelectValue /></SelectTrigger>
                          <SelectContent>
                            <SelectItem value="=">=</SelectItem>
                            <SelectItem value="!=">!=</SelectItem>
                            <SelectItem value=">">&gt;</SelectItem>
                            <SelectItem value=">=">&gt;=</SelectItem>
                            <SelectItem value="<">&lt;</SelectItem>
                            <SelectItem value="<=">&lt;=</SelectItem>
                            <SelectItem value="contains">包含</SelectItem>
                            <SelectItem value="in">∈</SelectItem>
                          </SelectContent>
                        </Select>
                        <Input defaultValue={c.value} className="h-7 text-xs flex-1" />
                        <span className="text-[10px] text-muted-foreground w-20 truncate">{f?.label}</span>
                        <Button variant="ghost" size="sm" className="h-6 w-6 p-0"><X className="h-3 w-3" /></Button>
                      </div>
                    );
                  })}
                </div>
                <Button variant="ghost" size="sm" className="h-6 mt-1.5 text-xs"><Plus className="h-3 w-3 mr-0.5" /> 加条件</Button>
              </div>
            ))}
          </Card>

          {/* 预览区 */}
          <Card className="p-3">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-1">
                <Button variant={chartType === "table" ? "default" : "outline"} size="sm" onClick={() => setChartType("table")}>
                  <TableIcon className="h-3.5 w-3.5 mr-1" /> 表格
                </Button>
                <Button variant={chartType === "bar" ? "default" : "outline"} size="sm" onClick={() => setChartType("bar")}>
                  <BarChart3 className="h-3.5 w-3.5 mr-1" /> 柱状
                </Button>
                <Button variant={chartType === "line" ? "default" : "outline"} size="sm" onClick={() => setChartType("line")}>
                  <LineChart className="h-3.5 w-3.5 mr-1" /> 折线
                </Button>
                <Button variant={chartType === "pie" ? "default" : "outline"} size="sm" onClick={() => setChartType("pie")}>
                  <PieChart className="h-3.5 w-3.5 mr-1" /> 饼图
                </Button>
                <Button variant={chartType === "area" ? "default" : "outline"} size="sm" onClick={() => setChartType("area")}>
                  <AreaChart className="h-3.5 w-3.5 mr-1" /> 面积
                </Button>
              </div>
              <span className="text-xs text-muted-foreground font-mono">查询用时 0.38s · 6 行 · MD5: a8f3…d2c1</span>
            </div>

            {chartType === "table" && <DataTable columns={cols} data={PREVIEW_DATA} rowKey={(r) => r.dim} />}

            {chartType === "bar" && (
              <div className="space-y-2 p-2">
                {PREVIEW_DATA.map((r) => (
                  <div key={r.dim} className="grid grid-cols-[140px_1fr_100px] gap-3 items-center text-xs">
                    <div className="truncate">{r.dim}</div>
                    <div className="h-5 bg-muted rounded relative overflow-hidden">
                      <div className="h-full bg-primary" style={{ width: `${(r.m3 / maxAmount) * 100}%` }} />
                    </div>
                    <div className="text-right font-mono">¥{r.m3.toLocaleString()}</div>
                  </div>
                ))}
              </div>
            )}

            {(chartType === "line" || chartType === "area") && (
              <div className="h-[200px] flex items-center justify-center bg-muted/20 rounded text-xs text-muted-foreground">
                <div className="text-center">
                  {chartType === "line" ? <LineChart className="h-8 w-8 mx-auto mb-2 opacity-40" /> : <AreaChart className="h-8 w-8 mx-auto mb-2 opacity-40" />}
                  <div>需"日期"维度才能显示{chartType === "line" ? "折线" : "面积"}图</div>
                  <div className="mt-0.5">从左侧拖"日期（日）"或"日期（月）"到"行"</div>
                </div>
              </div>
            )}

            {chartType === "pie" && (
              <div className="h-[260px] flex items-center justify-center gap-6">
                {/* 简易饼图（CSS 模拟）*/}
                <div className="relative w-[180px] h-[180px]">
                  <div className="absolute inset-0 rounded-full"
                       style={{
                         background: `conic-gradient(
                           hsl(var(--primary)) 0deg 90deg,
                           hsl(var(--wms-cold)) 90deg 180deg,
                           hsl(var(--wms-success)) 180deg 240deg,
                           hsl(var(--wms-warning)) 240deg 280deg,
                           hsl(var(--muted-foreground) / 0.5) 280deg 360deg
                         )`,
                       }} />
                  <div className="absolute inset-12 bg-background rounded-full flex items-center justify-center text-sm font-bold">
                    ¥925K
                  </div>
                </div>
                <div className="space-y-1.5 text-xs">
                  {PREVIEW_DATA.map((r, i) => {
                    const colors = ["bg-primary", "bg-wms-cold", "bg-wms-success", "bg-wms-warning", "bg-muted-foreground/50", "bg-muted-foreground/30"];
                    const pct = ((r.m3 / 925000) * 100).toFixed(1);
                    return (
                      <div key={r.dim} className="flex items-center gap-2">
                        <span className={`w-3 h-3 rounded ${colors[i]}`} />
                        <span className="w-32 truncate">{r.dim}</span>
                        <span className="font-mono text-muted-foreground">{pct}%</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </Card>

          {/* 已保存模板 */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <div className="text-sm font-semibold">已保存模板（5）</div>
              <span className="text-xs text-muted-foreground">私有 / 部门共享 / 全局 三档权限</span>
            </div>
            <div className="grid grid-cols-3 gap-2">
              {SAVED_TEMPLATES.map((t) => (
                <Card key={t.id} className="p-2.5 cursor-pointer hover:bg-muted/30">
                  <div className="flex items-start justify-between mb-1">
                    <div className="flex items-center gap-1.5 flex-1 min-w-0">
                      {t.isFavorite && <Star className="h-3.5 w-3.5 text-wms-warning fill-current flex-shrink-0" />}
                      <span className="text-xs font-medium truncate">{t.name}</span>
                    </div>
                    <Button variant="ghost" size="sm" className="h-5 w-5 p-0 flex-shrink-0">
                      <Trash2 className="h-3 w-3 text-muted-foreground" />
                    </Button>
                  </div>
                  <div className="flex items-center gap-2 text-[11px]">
                    <span className={`px-1 py-0.5 rounded ${
                      t.scope === "私有" ? "bg-muted text-muted-foreground" :
                      t.scope === "部门" ? "bg-primary/10 text-primary" :
                      "bg-wms-success/10 text-wms-success"
                    }`}>{t.scope}</span>
                    <span className="text-muted-foreground truncate">{t.lastRun}</span>
                  </div>
                </Card>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
