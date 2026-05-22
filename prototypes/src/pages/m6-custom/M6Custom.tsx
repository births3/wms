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
  type DataTableColumn,
} from "@/components/business";
import {
  Plus, Save, Star, BarChart3, LineChart, Table as TableIcon,
  Download, Trash2, Database,
} from "lucide-react";

/**
 * M6Custom — M6-003 业务报表（自定义查询）
 *
 * 层级：Layer 3 页面级
 * 关联故事：US-M6-003（自定义维度 + 指标 + 表格/图表 + 模板）
 * Wave：Wave 3.0（M6 报表合规）
 * 业务约束：自定义报表不替代 GSP 法定报表；保存的模板用户私有；运营查询写 H2 审计
 *
 * @example
 *   <M6Custom />
 */

interface PreviewRow {
  dim: string;
  metric1: number;
  metric2: number;
  metric3: number;
  metric4: number;
}

const PREVIEW_DATA: PreviewRow[] = [
  { dim: "国药控股北京", metric1: 12, metric2: 1820, metric3: 215400, metric4: 0 },
  { dim: "上海医药华东", metric1: 9, metric2: 1320, metric3: 168200, metric4: 1 },
  { dim: "九州通医药",   metric1: 8, metric2: 980,  metric3: 92400,  metric4: 0 },
  { dim: "甘李药业",     metric1: 5, metric2: 580,  metric3: 286800, metric4: 0 },
  { dim: "东北制药",     metric1: 4, metric2: 480,  metric3: 24600,  metric4: 0 },
  { dim: "其他（12 家）", metric1: 18, metric2: 2240, metric3: 137200, metric4: 2 },
];

const SAVED_TEMPLATES = [
  { id: "t1", name: "供应商月度采购排行", desc: "维度=供应商 / 指标=单数+件数+金额", lastRun: "今日 09:14", isFavorite: true },
  { id: "t2", name: "冷链商品销售趋势", desc: "维度=日期 / 商品分类=冷链 / 指标=件数+金额", lastRun: "昨日 16:30", isFavorite: true },
  { id: "t3", name: "麻精药品月度核对", desc: "维度=月+商品 / 商品分类=麻精 / 指标=入出库+库存", lastRun: "5/15 10:08" },
  { id: "t4", name: "客户退货率分析", desc: "维度=客户 / 指标=出库件数+退货件数+退货率", lastRun: "5/12 14:22" },
];

export function M6Custom() {
  const [chartType, setChartType] = useState<"table" | "bar" | "line">("table");

  const cols: DataTableColumn<PreviewRow>[] = [
    { key: "dim", header: "供应商", render: (r) => <span className="text-sm font-medium">{r.dim}</span> },
    { key: "m1", header: "入库单数", align: "right", render: (r) => <span className="text-sm">{r.metric1}</span> },
    { key: "m2", header: "总件数", align: "right", render: (r) => <span className="text-sm">{r.metric2.toLocaleString()}</span> },
    { key: "m3", header: "总金额", align: "right", render: (r) => <span className="text-sm font-mono">¥{r.metric3.toLocaleString()}</span> },
    { key: "m4", header: "异常单", align: "right", render: (r) =>
      r.metric4 > 0 ? <span className="text-sm text-destructive">{r.metric4}</span> :
      <span className="text-sm text-muted-foreground">—</span>
    },
  ];

  // 简单的横向柱状图（基于 metric3 金额）
  const maxAmount = Math.max(...PREVIEW_DATA.map((r) => r.metric3));

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="业务报表（自定义）"
        subtitle="M6-003 · 多维度查询 + 灵活指标 + 图表预览 + 模板复用"
        actions={
          <>
            <Button variant="outline" size="sm">
              <Download className="h-4 w-4 mr-1" /> 导出
            </Button>
            <Button size="sm">
              <Save className="h-4 w-4 mr-1" /> 保存为模板
            </Button>
          </>
        }
      />

      <div className="px-6 py-4 grid grid-cols-[320px_1fr] gap-4">
        {/* 左侧：报表配置 */}
        <div className="space-y-3">
          <Card className="p-3">
            <div className="text-xs font-semibold text-muted-foreground mb-2">数据源</div>
            <Select defaultValue="purchase">
              <SelectTrigger className="h-8 text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="purchase">采购入库</SelectItem>
                <SelectItem value="sales">销售出库</SelectItem>
                <SelectItem value="inventory">库存</SelectItem>
                <SelectItem value="cold">冷链温度</SelectItem>
                <SelectItem value="quality">质量管理</SelectItem>
              </SelectContent>
            </Select>
          </Card>

          <Card className="p-3">
            <div className="text-xs font-semibold text-muted-foreground mb-2">维度（分组）</div>
            <div className="space-y-1.5">
              {[
                { v: "supplier", l: "供应商", checked: true },
                { v: "date_month", l: "日期（月）" },
                { v: "date_day", l: "日期（日）" },
                { v: "warehouse", l: "仓库" },
                { v: "owner", l: "货主" },
                { v: "category", l: "商品分类" },
              ].map((d) => (
                <label key={d.v} className="flex items-center gap-2 text-xs cursor-pointer">
                  <input type="checkbox" defaultChecked={d.checked} className="accent-primary" />
                  <span>{d.l}</span>
                </label>
              ))}
            </div>
          </Card>

          <Card className="p-3">
            <div className="text-xs font-semibold text-muted-foreground mb-2">指标</div>
            <div className="space-y-1.5">
              {[
                { v: "asn_count", l: "入库单数", checked: true },
                { v: "qty", l: "总件数", checked: true },
                { v: "amount", l: "总金额", checked: true },
                { v: "exception", l: "异常单数", checked: true },
                { v: "rejected", l: "拒收单数" },
                { v: "avg_amount", l: "平均单价" },
              ].map((m) => (
                <label key={m.v} className="flex items-center gap-2 text-xs cursor-pointer">
                  <input type="checkbox" defaultChecked={m.checked} className="accent-primary" />
                  <span>{m.l}</span>
                </label>
              ))}
            </div>
          </Card>

          <Card className="p-3">
            <div className="text-xs font-semibold text-muted-foreground mb-2">过滤条件</div>
            <div className="space-y-2">
              <div className="grid grid-cols-2 gap-1.5 text-xs">
                <div className="text-muted-foreground self-center">日期范围</div>
                <Input type="date" defaultValue="2026-04-01" className="h-7" />
              </div>
              <div className="grid grid-cols-2 gap-1.5 text-xs">
                <div className="text-muted-foreground self-center">至</div>
                <Input type="date" defaultValue="2026-04-30" className="h-7" />
              </div>
              <Button variant="outline" size="sm" className="h-7 w-full text-xs">
                <Plus className="h-3 w-3 mr-1" /> 添加过滤条件
              </Button>
            </div>
          </Card>
        </div>

        {/* 右侧：预览 */}
        <div className="min-w-0">
          {/* 工具栏 */}
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-1">
              <Button
                variant={chartType === "table" ? "default" : "outline"}
                size="sm"
                onClick={() => setChartType("table")}
              >
                <TableIcon className="h-3.5 w-3.5 mr-1" /> 表格
              </Button>
              <Button
                variant={chartType === "bar" ? "default" : "outline"}
                size="sm"
                onClick={() => setChartType("bar")}
              >
                <BarChart3 className="h-3.5 w-3.5 mr-1" /> 柱状图
              </Button>
              <Button
                variant={chartType === "line" ? "default" : "outline"}
                size="sm"
                onClick={() => setChartType("line")}
              >
                <LineChart className="h-3.5 w-3.5 mr-1" /> 折线图
              </Button>
            </div>
            <span className="text-xs text-muted-foreground font-mono">查询用时 0.38s · 6 行 · MD5: a8f3…d2c1</span>
          </div>

          {/* 表格预览 */}
          {chartType === "table" && (
            <DataTable columns={cols} data={PREVIEW_DATA} rowKey={(r) => r.dim} />
          )}

          {/* 柱状图预览（横向 bar） */}
          {chartType === "bar" && (
            <Card className="p-4">
              <div className="text-sm font-medium mb-3">按供应商金额（柱状图）</div>
              <div className="space-y-2">
                {PREVIEW_DATA.map((r) => (
                  <div key={r.dim} className="grid grid-cols-[140px_1fr_100px] gap-3 items-center text-xs">
                    <div className="truncate">{r.dim}</div>
                    <div className="h-5 bg-muted rounded relative overflow-hidden">
                      <div
                        className="h-full bg-primary"
                        style={{ width: `${(r.metric3 / maxAmount) * 100}%` }}
                      />
                    </div>
                    <div className="text-right font-mono">¥{r.metric3.toLocaleString()}</div>
                  </div>
                ))}
              </div>
            </Card>
          )}

          {/* 折线图预览（占位） */}
          {chartType === "line" && (
            <Card className="p-4 h-[260px] flex items-center justify-center bg-muted/30">
              <div className="text-center text-xs text-muted-foreground">
                <LineChart className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <div>需要"日期"维度才能显示折线图</div>
                <div className="mt-1">在左侧维度勾选"日期（日）"或"日期（月）"</div>
              </div>
            </Card>
          )}

          {/* 已保存模板 */}
          <div className="mt-6">
            <div className="flex items-center justify-between mb-2">
              <div className="text-sm font-semibold">已保存模板（4）</div>
              <span className="text-xs text-muted-foreground">私有 · 仅自己可见</span>
            </div>
            <div className="grid grid-cols-2 gap-2">
              {SAVED_TEMPLATES.map((t) => (
                <Card key={t.id} className="p-3 cursor-pointer hover:bg-muted/30">
                  <div className="flex items-start justify-between mb-1">
                    <div className="flex items-center gap-1.5">
                      {t.isFavorite && <Star className="h-3.5 w-3.5 text-wms-warning fill-current" />}
                      <span className="text-sm font-medium">{t.name}</span>
                    </div>
                    <Button variant="ghost" size="sm" className="h-6 w-6 p-0">
                      <Trash2 className="h-3 w-3 text-muted-foreground" />
                    </Button>
                  </div>
                  <div className="text-[11px] text-muted-foreground">{t.desc}</div>
                  <div className="text-[11px] text-muted-foreground mt-1.5 flex items-center gap-1">
                    <Database className="h-3 w-3" /> 上次运行 {t.lastRun}
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
