import { useState } from "react";
import { Button, Card, DataTable, type DataTableColumn } from "@wms/ui";
import { AreaChart, BarChart3, LineChart, PieChart, Table as TableIcon } from "lucide-react";
import { PREVIEW_DATA, type ChartType, type PreviewRow } from "./m6-custom-data";

/**
 * M6CustomPreview — 自定义业务报表预览区
 *
 * 层级：Layer 3 页面级子组件
 * 关联故事：US-M6-003（自定义维度+指标+图表+保存模板+订阅+Metabase 嵌入）
 * Wave：Wave 0.5（演示原型）→ Wave 5（Metabase 嵌入正式上线）
 *
 * @example
 *   <M6CustomPreview />
 */
export function M6CustomPreview() {
  const [chartType, setChartType] = useState<ChartType>("table");
  const maxAmount = Math.max(...PREVIEW_DATA.map((row) => row.m3));

  const cols: DataTableColumn<PreviewRow>[] = [
    { key: "dim", header: "供应商（行）", render: (row) => <span className="text-sm font-medium">{row.dim}</span> },
    { key: "m1", header: "Σ 入库单数", align: "right", render: (row) => <span className="text-sm">{row.m1}</span> },
    { key: "m2", header: "Σ 总件数", align: "right", render: (row) => <span className="text-sm">{row.m2.toLocaleString()}</span> },
    { key: "m3", header: "Σ 总金额", align: "right", render: (row) => <span className="text-sm font-mono">¥{row.m3.toLocaleString()}</span> },
  ];

  return (
    <Card className="p-3">
      <div className="flex items-center justify-between gap-3 mb-2">
        <div className="flex items-center gap-1 flex-wrap">
          <ChartButton active={chartType === "table"} onClick={() => setChartType("table")} icon={<TableIcon data-icon="inline-start" />} label="表格" />
          <ChartButton active={chartType === "bar"} onClick={() => setChartType("bar")} icon={<BarChart3 data-icon="inline-start" />} label="柱状" />
          <ChartButton active={chartType === "line"} onClick={() => setChartType("line")} icon={<LineChart data-icon="inline-start" />} label="折线" />
          <ChartButton active={chartType === "pie"} onClick={() => setChartType("pie")} icon={<PieChart data-icon="inline-start" />} label="饼图" />
          <ChartButton active={chartType === "area"} onClick={() => setChartType("area")} icon={<AreaChart data-icon="inline-start" />} label="面积" />
        </div>
        <span className="text-xs text-muted-foreground font-mono">M6-003 · GSP · 查询 0.38s · 6 行 · MD5: a8f3-d2c1</span>
      </div>

      {chartType === "table" && <DataTable columns={cols} data={PREVIEW_DATA} rowKey={(row) => row.dim} />}
      {chartType === "bar" && <BarPreview maxAmount={maxAmount} />}
      {(chartType === "line" || chartType === "area") && <TimelinePlaceholder chartType={chartType} />}
      {chartType === "pie" && <PiePreview />}
    </Card>
  );
}

function ChartButton(props: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return (
    <Button variant={props.active ? "default" : "outline"} size="sm" onClick={props.onClick}>
      {props.icon}
      {props.label}
    </Button>
  );
}

function BarPreview({ maxAmount }: { maxAmount: number }) {
  return (
    <div className="flex flex-col gap-2 p-2">
      {PREVIEW_DATA.map((row) => (
        <div key={row.dim} className="grid grid-cols-[140px_1fr_100px] gap-3 items-center text-xs">
          <div className="truncate">{row.dim}</div>
          <div className="h-5 bg-muted rounded relative overflow-hidden">
            <div className="h-full bg-primary" style={{ width: `${(row.m3 / maxAmount) * 100}%` }} />
          </div>
          <div className="text-right font-mono">¥{row.m3.toLocaleString()}</div>
        </div>
      ))}
    </div>
  );
}

function TimelinePlaceholder({ chartType }: { chartType: Extract<ChartType, "line" | "area"> }) {
  const Icon = chartType === "line" ? LineChart : AreaChart;
  return (
    <div className="h-[200px] flex items-center justify-center bg-muted/20 rounded text-xs text-muted-foreground">
      <div className="text-center">
        <Icon className="size-8 mx-auto mb-2 opacity-40" />
        <div>需"日期"维度才能显示{chartType === "line" ? "折线" : "面积"}图</div>
        <div className="mt-0.5">从左侧拖"日期（日）"或"日期（月）"到"行"</div>
      </div>
    </div>
  );
}

function PiePreview() {
  const colors = ["bg-primary", "bg-wms-cold", "bg-wms-success", "bg-wms-warning", "bg-muted-foreground/50", "bg-muted-foreground/30"];

  return (
    <div className="h-[260px] flex items-center justify-center gap-6">
      <div className="relative w-[180px] h-[180px]">
        <div
          className="absolute inset-0 rounded-full"
          style={{
            background: `conic-gradient(
              hsl(var(--primary)) 0deg 90deg,
              hsl(var(--wms-cold)) 90deg 180deg,
              hsl(var(--wms-success)) 180deg 240deg,
              hsl(var(--wms-warning)) 240deg 280deg,
              hsl(var(--muted-foreground) / 0.5) 280deg 360deg
            )`,
          }}
        />
        <div className="absolute inset-12 bg-background rounded-full flex items-center justify-center text-sm font-bold">
          ¥925K
        </div>
      </div>
      <div className="flex flex-col gap-1.5 text-xs">
        {PREVIEW_DATA.map((row, index) => (
          <div key={row.dim} className="flex items-center gap-2">
            <span className={`size-3 rounded ${colors[index]}`} />
            <span className="w-32 truncate">{row.dim}</span>
            <span className="font-mono text-muted-foreground">{((row.m3 / 925000) * 100).toFixed(1)}%</span>
          </div>
        ))}
      </div>
    </div>
  );
}
