import {
  ApprovalFlow,
  AuditTimeline,
  Card,
  CardContent,
  DiffPanel,
  FieldTable,
  PrintPreview,
  RuleEditor,
  StatusBadge,
  TempChart,
  type AuditTimelineEvent,
  type RuleAction,
  type RuleGroup,
  type TempPoint,
} from "@wms/ui";
import { CheckCircle2 } from "lucide-react";
import type { MatrixPrototypeSpec } from "./types";
import type { MetricItem, StoryPrototypeModel } from "./prototype-model";

export function PrototypeSidePanel({ spec, model }: { spec: MatrixPrototypeSpec; model: StoryPrototypeModel }) {
  const events: AuditTimelineEvent[] = model.auditEvents.map((action, idx) => ({
    id: `e${idx + 1}`,
    time: `2026-05-24 09:${String(10 + idx * 8).padStart(2, "0")}:12`,
    actor: idx === 2 ? "系统" : `u00${idx + 1} ${idx === 0 ? "张三" : "李四"}`,
    action,
    module: idx === model.auditEvents.length - 1 ? "H2" : spec.moduleCode,
    resource: idx === 0 ? spec.storyId : `${spec.slug}.${idx}`,
    status: idx < 2 ? "completed" : idx === 2 ? "in_progress" : "pending",
  }));

  return (
    <div className="flex flex-col gap-4">
      <Card className="rounded-md">
        <CardContent className="p-4">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-sm font-semibold">原型走查要点</span>
            <StatusBadge status="qualified" size="sm" label={spec.priority} />
          </div>
          <div className="flex flex-col gap-2 text-sm">
            <ChecklistItem text={`${spec.storyId} 绑定 ${model.primaryObject}`} />
            <ChecklistItem text={`${spec.end.toUpperCase()} 端字段与流程来自故事模型`} />
            <ChecklistItem text={`${model.moduleName} 异常路径已可见`} />
            <ChecklistItem text="H1 权限 / H2 审计 / H3 契约占位齐全" />
          </div>
        </CardContent>
      </Card>

      <Card className="rounded-md">
        <CardContent className="p-4">
          <div className="mb-3 text-sm font-semibold">状态变化</div>
          <DiffPanel layout="stacked" before={model.before} after={model.after} />
        </CardContent>
      </Card>

      <Card className="rounded-md">
        <CardContent className="p-4">
          <div className="mb-3 text-sm font-semibold">审计时间线</div>
          <AuditTimeline events={events} expandedId="e2" />
        </CardContent>
      </Card>
    </div>
  );
}

export function PrintArea({ spec, model }: { spec: MatrixPrototypeSpec; model: StoryPrototypeModel }) {
  return (
    <PrintPreview template={spec.title.includes("面单") ? "shipping" : "a4"} zoom={0.7} pageCount={2}>
      <div className="flex flex-col gap-4 text-xs">
        <div className="text-center">
          <div className="text-lg font-semibold">{spec.title}</div>
          <div className="text-[10px] text-muted-foreground">{spec.storyId} · {model.primaryObject} · GSP 单据格式</div>
        </div>
        <FieldTable rows={model.fields.slice(0, 5)} size="sm" />
        <div className="grid grid-cols-2 gap-6 pt-8">
          <div className="border-t pt-2">复核人：u001 张三</div>
          <div className="border-t pt-2">交接人：u002 李四</div>
        </div>
      </div>
    </PrintPreview>
  );
}

export function TemperatureArea({ spec, model }: { spec: MatrixPrototypeSpec; model: StoryPrototypeModel }) {
  const points: TempPoint[] = [
    { t: "08:00", v: 4.8 },
    { t: "10:00", v: 5.1 },
    { t: "12:00", v: 8.7 },
    { t: "14:00", v: 7.2 },
    { t: "16:00", v: 5.9 },
    { t: "18:00", v: 4.9 },
    { t: "20:00", v: 3.8 },
  ];
  return (
    <Card className="rounded-md">
      <CardContent className="p-4">
        <TempChart points={points} minThreshold={2} maxThreshold={8} />
        <div className="mt-4 grid gap-3 text-sm md:grid-cols-3">
          <Metric label="关联对象" value={model.primaryObject} />
          <Metric label="超阈值" value="1 次" />
          <Metric label="来源故事" value={spec.storyId} />
        </div>
      </CardContent>
    </Card>
  );
}

export function RuleArea({ spec, model }: { spec: MatrixPrototypeSpec; model: StoryPrototypeModel }) {
  const groups: RuleGroup[] = [
    { conditions: [{ field: model.fields[0]?.label ?? "object", op: "eq", value: String(model.fields[0]?.value ?? spec.storyId) }] },
    { conditions: [{ field: model.fields[2]?.label ?? "status", op: "in", value: model.exceptions.join(",") }] },
  ];
  const actions: RuleAction[] = model.actions.slice(0, 2).map((action, idx) => ({
    type: idx === 0 ? "create_alert" : "require_approval",
    label: action,
    params: { module: spec.moduleCode, story: spec.storyId },
  }));
  return (
    <Card className="rounded-md">
      <CardContent className="p-4">
        <div className="mb-3 text-sm font-semibold">{spec.title} · 规则预览</div>
        <RuleEditor groups={groups} actions={actions} />
      </CardContent>
    </Card>
  );
}

export function ApprovalArea({ spec, model }: { spec: MatrixPrototypeSpec; model: StoryPrototypeModel }) {
  return (
    <Card className="rounded-md">
      <CardContent className="p-4">
        <div className="mb-3 text-sm font-semibold">{spec.title} · 审批链路</div>
        <ApprovalFlow
          nodes={[
            { role: "发起人", approver: "u001 张三", time: "09:10", status: "approved" },
            { role: "质量负责人", approver: "u002 李四", time: "09:18", status: "current", comment: model.exceptions[0] ?? "核对批号与库存状态" },
            { role: "仓库主管", status: "pending" },
          ]}
        />
      </CardContent>
    </Card>
  );
}

function Metric({ label, value }: MetricItem) {
  return (
    <div className="rounded-md border bg-muted/30 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 font-semibold">{value}</div>
    </div>
  );
}

function ChecklistItem({ text }: { text: string }) {
  return (
    <div className="flex items-center gap-2">
      <CheckCircle2 className="size-4 text-wms-success" />
      <span>{text}</span>
    </div>
  );
}
