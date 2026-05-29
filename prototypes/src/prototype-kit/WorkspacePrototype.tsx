import {
  Button,
  DataTable,
  Input,
  KanbanBoard,
  PageHeader,
  StatusBadge,
  type DataTableColumn,
  type KanbanColumn,
} from "@wms/ui";
import {
  Bell,
  Boxes,
  CalendarClock,
  ClipboardCheck,
  Download,
  FileText,
  Filter,
  Gauge,
  PackageCheck,
  Plus,
  RefreshCcw,
  Search,
  ShieldCheck,
  Truck,
  Workflow,
} from "lucide-react";
import type { MatrixPrototypeSpec } from "./types";
import { isApproval } from "./prototype-classifiers";
import {
  buildStoryPrototypeModel,
  type PrototypeRow,
  type StoryPrototypeModel,
} from "./prototype-model";
import {
  ApprovalArea,
  PrintArea,
  PrototypeSidePanel,
  RuleArea,
  TemperatureArea,
} from "./PrototypePanels";

const MODULE_ICON: Record<string, React.ComponentType<{ className?: string }>> = {
  AL: Bell,
  BA: ClipboardCheck,
  DI: FileText,
  DOCK: CalendarClock,
  DR: Truck,
  H1: ShieldCheck,
  H2: FileText,
  H3: Workflow,
  H4: Bell,
  H5: Truck,
  M1: Boxes,
  M2: PackageCheck,
  M3: Gauge,
  M4: PackageCheck,
  M5: Gauge,
  M6: FileText,
  M8: Boxes,
  M9: FileText,
  M10: Truck,
  MPM: Workflow,
  PK: PackageCheck,
  QL: ClipboardCheck,
  RC: RefreshCcw,
  RP: Boxes,
  SA: ClipboardCheck,
  ST: Boxes,
  TC: Workflow,
  TE: Workflow,
  VR: Workflow,
};

/**
 * WorkspacePrototype — PC/PAD 全量矩阵工作台原型
 *
 * 层级：Layer 3 页面级支撑组件
 * 关联故事：docs/prototypes/prototype-matrix-r3.md 中 PC/PAD story/end
 * Wave：Wave 0.5+ 全量原型补齐
 * 业务约束：表格字段、筛选、流程与异常必须来自 StoryPrototypeModel
 *
 * @example
 *   <WorkspacePrototype spec={spec} mode="pc" />
 */
export function WorkspacePrototype({ spec, mode }: { spec: MatrixPrototypeSpec; mode: "pc" | "pad" }) {
  const model = buildStoryPrototypeModel(spec);
  const Icon = MODULE_ICON[spec.moduleCode] ?? Workflow;
  const columns = makeColumns(model);

  return (
    <div className={mode === "pad" ? "w-full max-w-[1180px] min-w-0" : "w-full max-w-[1440px] min-w-0"}>
      <div className="overflow-hidden rounded-lg border bg-background shadow-sm">
        <PageHeader
          title={spec.title}
          subtitle={`${spec.storyId} · ${model.moduleName} · ${spec.reason}`}
          breadcrumb={`${spec.group} / ${spec.end.toUpperCase()} / ${spec.priority}`}
          actions={
            <>
              {model.actions.slice(0, 3).map((action, idx) => (
                <Button key={action} variant={idx === 2 ? "default" : "outline"} size="sm">
                  {idx === 0 ? <Download data-icon="inline-start" /> : idx === 1 ? <RefreshCcw data-icon="inline-start" /> : <Plus data-icon="inline-start" />}
                  {action}
                </Button>
              ))}
            </>
          }
        />

        <div className="border-y bg-muted/30 px-4 py-4 sm:px-6">
          <div className={mode === "pad" ? "grid grid-cols-2 gap-3 lg:grid-cols-3" : "grid grid-cols-2 gap-3 lg:grid-cols-5"}>
            {model.stats.map((item) => (
              <div key={item.label} className="rounded-md border bg-background p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-xs text-muted-foreground">{item.label}</span>
                  <Icon className="size-4 shrink-0 text-primary" />
                </div>
                <div className="mt-2 text-xl font-semibold">{item.value}</div>
                <div className="mt-1 text-xs text-muted-foreground">{item.hint}</div>
              </div>
            ))}
          </div>
        </div>

        <div className="border-b bg-background px-4 py-4 sm:px-6">
          <div className="grid gap-3 md:grid-cols-[1.5fr_1fr_1fr_1fr_auto] md:items-end">
            <div>
              <label className="mb-1 block text-xs text-muted-foreground">{model.primaryObject}关键字</label>
              <div className="relative">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input className="pl-9" placeholder={model.searchPlaceholder} />
              </div>
            </div>
            {model.filters.map((filter) => (
              <FilterSelect key={filter.label} label={filter.label} value={String(filter.value)} />
            ))}
            <Button variant="outline">
              <Filter className="size-4" />
              筛选
            </Button>
          </div>
        </div>

        <div className="grid gap-4 p-4 xl:grid-cols-[minmax(0,1fr)_360px] sm:p-6">
          <div className="min-w-0 flex flex-col gap-4">
            <MainWorkArea spec={spec} model={model} columns={columns} />
            {isApproval(spec) && <ApprovalArea spec={spec} model={model} />}
          </div>
          <PrototypeSidePanel spec={spec} model={model} />
        </div>
      </div>
    </div>
  );
}

function MainWorkArea({
  spec,
  model,
  columns,
}: {
  spec: MatrixPrototypeSpec;
  model: StoryPrototypeModel;
  columns: DataTableColumn<PrototypeRow>[];
}) {
  if (model.layoutKind === "kanban") return <KanbanBoard columns={makeKanban(model)} />;
  if (model.layoutKind === "print") return <PrintArea spec={spec} model={model} />;
  if (model.layoutKind === "temperature") return <TemperatureArea spec={spec} model={model} />;
  if (model.layoutKind === "rule") return <RuleArea spec={spec} model={model} />;
  return (
    <DataTable<PrototypeRow>
      columns={columns}
      data={model.rows}
      rowKey={(row) => row.id}
      selectedKey={model.rows[0]?.id}
      caption={`${spec.storyId} · ${model.primaryObject} · 共 ${model.rows.length} 条`}
      footer={
        <div className="flex items-center justify-between px-4 py-2 text-xs text-muted-foreground">
          <span>每页 20 条 · 第 1 / 8 页</span>
          <span>写操作进入 H2 append-only 审计链</span>
        </div>
      }
    />
  );
}

function FilterSelect({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <label className="mb-1 block text-xs text-muted-foreground">{label}</label>
      <div className="flex h-9 items-center rounded-md border bg-background px-3 text-sm">{value}</div>
    </div>
  );
}

function makeColumns(model: StoryPrototypeModel): DataTableColumn<PrototypeRow>[] {
  return model.columns.map((col) => ({
    key: col.key,
    header: col.header,
    align: col.align,
    mono: col.mono,
    render: col.key === "status" ? (row) => <StatusBadge status={statusFor(row.status)} size="sm" label={row.status} /> : undefined,
  }));
}

function makeKanban(model: StoryPrototypeModel): KanbanColumn[] {
  return [
    { title: "待处理", variant: "warning", items: model.rows.slice(0, 2).map((row) => kanbanItem(row, model)) },
    { title: "进行中", items: model.rows.slice(2, 4).map((row) => kanbanItem(row, model)) },
    { title: "已完成", variant: "success", items: model.rows.slice(4, 6).map((row) => kanbanItem(row, model)) },
  ];
}

function kanbanItem(row: PrototypeRow, model: StoryPrototypeModel) {
  return {
    id: row.id,
    title: row.c0 ?? model.primaryObject,
    subtitle: row.c1 ?? model.moduleName,
    priority: row.status === "异常" ? "urgent" : "normal",
    status: statusFor(row.status),
    meta: [
      { label: model.columns[2]?.header ?? "对象", value: row.c2 ?? "-" },
      { label: model.columns[3]?.header ?? "进度", value: row.c3 ?? "-" },
    ],
  };
}

function statusFor(status: string) {
  if (/异常|失败|不合格/.test(status)) return "unqualified";
  if (/完成|归档/.test(status)) return "completed";
  if (/复核|审批|待/.test(status)) return "pending";
  return "in_progress";
}
