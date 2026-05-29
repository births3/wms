import { useState } from "react";
import { Button, Card, Input, PageHeader, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@wms/ui";
import { Database, Download, ExternalLink, GripVertical, Plus, Save, X } from "lucide-react";
import { ALL_FIELDS, INITIAL_FILTERS, VALUE_METRICS } from "./m6-custom-data";
import { M6CustomPreview } from "./M6CustomPreview";
import { M6CustomTemplates } from "./M6CustomTemplates";

/**
 * M6CustomBuilder — M6-003 自定义业务报表搭建器
 *
 * 层级：Layer 3 页面级子组件
 * 关联故事：US-M6-003（自定义维度+指标+图表+保存模板+订阅+Metabase 嵌入）
 * Wave：Wave 0.5（演示原型）→ Wave 5（Metabase 嵌入正式上线）
 * 业务约束：仿 Metabase 拖拽体验；不替代 GSP 法定报表（M6-001）；查询写 H2 审计
 *
 * @example
 *   <M6CustomBuilder />
 */
export function M6CustomBuilder() {
  const [reportName, setReportName] = useState("供应商月度采购排行（草稿）");

  return (
    <div className="w-full max-w-[1400px] bg-background rounded-lg border shadow-sm">
      <PageHeader
        title="业务报表（自建）"
        subtitle="M6-003 · US-M6-003 · GSP audit trail · Metabase ADR-0023 · Wave 5"
        actions={<HeaderActions />}
      />

      <ReportMetaBar reportName={reportName} onReportNameChange={setReportName} />

      <div className="px-6 py-4 grid grid-cols-[280px_1fr] gap-4">
        <FieldDrawer />
        <div className="flex flex-col gap-3">
          <BucketGrid />
          <FilterBuilder />
          <M6CustomPreview />
          <M6CustomTemplates />
        </div>
      </div>
    </div>
  );
}

function HeaderActions() {
  return (
    <>
      <Button variant="outline" size="sm">
        <ExternalLink data-icon="inline-start" /> 在 Metabase 中打开
      </Button>
      <Button variant="outline" size="sm">
        <Download data-icon="inline-start" /> 导出
      </Button>
      <Button size="sm">
        <Save data-icon="inline-start" /> 保存模板
      </Button>
    </>
  );
}

function ReportMetaBar(props: { reportName: string; onReportNameChange: (name: string) => void }) {
  return (
    <div className="px-6 py-3 border-b flex items-center gap-3">
      <Input value={props.reportName} onChange={(event) => props.onReportNameChange(event.target.value)} className="max-w-[300px] h-8 font-medium" />
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
      <span className="ml-auto text-xs text-muted-foreground font-mono">M6-003 / GSP / ADR-0023 / MD5 / 最后保存 5 分钟前</span>
    </div>
  );
}

function FieldDrawer() {
  return (
    <div className="flex flex-col gap-3">
      <Card className="p-3">
        <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1">
          <Database data-icon="inline-start" /> 可用字段（拖到右侧）
        </div>
        <FieldGroup title="维度" type="dim" badge="D" badgeClassName="bg-primary/10 text-primary" />
        <FieldGroup title="指标" type="metric" badge="M" badgeClassName="bg-wms-success/10 text-wms-success" />
      </Card>
    </div>
  );
}

function FieldGroup(props: { title: string; type: "dim" | "metric"; badge: string; badgeClassName: string }) {
  return (
    <>
      <div className="text-[11px] text-muted-foreground mb-2 italic">{props.title}</div>
      <div className={props.type === "dim" ? "flex flex-col gap-1 mb-3" : "flex flex-col gap-1"}>
        {ALL_FIELDS.filter((field) => field.type === props.type).map((field) => (
          <div key={field.id} className="flex items-center gap-2 px-2 py-1.5 bg-muted/30 rounded text-xs cursor-move hover:bg-primary/10">
            <GripVertical className="size-3 text-muted-foreground flex-shrink-0" />
            <span className="flex-1">{field.label}</span>
            <span className={`text-[10px] px-1 py-0.5 rounded ${props.badgeClassName}`}>{props.badge}</span>
          </div>
        ))}
      </div>
    </>
  );
}

function BucketGrid() {
  return (
    <div className="grid grid-cols-3 gap-3">
      <BucketCard title="行（分组）">
        <BucketChip label="供应商" className="bg-primary/10 text-primary" />
      </BucketCard>
      <BucketCard title="列（透视）">
        <div className="text-[11px] text-muted-foreground italic px-2">拖入维度展开为列</div>
      </BucketCard>
      <BucketCard title="值（指标聚合）">
        {VALUE_METRICS.map((metric) => (
          <BucketChip key={metric.label} label={metric.label} className="bg-wms-success/10 text-wms-success" />
        ))}
      </BucketCard>
    </div>
  );
}

function BucketCard(props: { title: string; children: React.ReactNode }) {
  return (
    <Card className="p-3">
      <div className="text-xs font-semibold mb-2">{props.title}</div>
      <div className="flex flex-col gap-1.5 min-h-[80px] bg-muted/20 p-2 rounded border-2 border-dashed">{props.children}</div>
    </Card>
  );
}

function BucketChip(props: { label: string; className: string }) {
  return (
    <div className={`flex items-center gap-2 px-2 py-1.5 rounded text-xs ${props.className}`}>
      <GripVertical className="size-3" />
      <span className="flex-1 font-medium">{props.label}</span>
      <X className="size-3 cursor-pointer" />
    </div>
  );
}

function FilterBuilder() {
  return (
    <Card className="p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="text-xs font-semibold">过滤条件</div>
        <Button variant="ghost" size="sm" className="h-6 text-xs"><Plus data-icon="inline-start" /> 加条件组</Button>
      </div>
      {INITIAL_FILTERS.map((group) => (
        <div key={group.id} className="border rounded p-2 bg-muted/20">
          <div className="flex items-center gap-2 mb-1.5">
            <Select defaultValue={group.connector}>
              <SelectTrigger className="h-7 w-[80px] text-xs"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem value="AND">AND</SelectItem>
                <SelectItem value="OR">OR</SelectItem>
              </SelectContent>
            </Select>
            <span className="text-xs text-muted-foreground">同时满足以下 {group.conditions.length} 个条件</span>
          </div>
          <div className="flex flex-col gap-1.5">
            {group.conditions.map((condition) => <FilterConditionRow key={condition.id} condition={condition} />)}
          </div>
          <Button variant="ghost" size="sm" className="h-6 mt-1.5 text-xs"><Plus data-icon="inline-start" /> 加条件</Button>
        </div>
      ))}
    </Card>
  );
}

function FilterConditionRow({ condition }: { condition: { field: string; op: string; value: string } }) {
  const field = ALL_FIELDS.find((candidate) => candidate.id === condition.field);

  return (
    <div className="flex items-center gap-2 text-xs">
      <Select defaultValue={condition.field}>
        <SelectTrigger className="h-7 w-[140px] text-xs"><SelectValue /></SelectTrigger>
        <SelectContent>
          {ALL_FIELDS.map((candidate) => <SelectItem key={candidate.id} value={candidate.id}>{candidate.label}</SelectItem>)}
        </SelectContent>
      </Select>
      <Select defaultValue={condition.op}>
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
      <Input defaultValue={condition.value} className="h-7 text-xs flex-1" />
      <span className="text-[10px] text-muted-foreground w-20 truncate">{field?.label}</span>
      <Button variant="ghost" size="sm" className="size-6 p-0"><X className="size-3" /></Button>
    </div>
  );
}
