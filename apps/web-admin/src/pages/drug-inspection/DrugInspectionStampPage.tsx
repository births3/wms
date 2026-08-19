import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  Input,
  ListPageTemplate,
  StatusBadge,
  cn,
  type DataGridColumn,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Check, FileImage, Send, X } from "lucide-react";

import {
  type DrugInspectionCustomerCopyJob,
  type DrugInspectionStampVersion,
  useApproveDrugInspectionCopyOversizeMutation,
  useCreateDrugInspectionStampMutation,
  useDrugInspectionCopyJobsQuery,
  useDrugInspectionProcessingRulesQuery,
  useDrugInspectionStampVersionsQuery,
  usePublishDrugInspectionProcessingRuleMutation,
  useReviewDrugInspectionStampMutation,
} from "@/features/drug-inspection/stamp-queries";
import { COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_UPDATED_AT, COLUMN_VERSION, FIELD_KEYWORD, FILTER_ALL, STATUS_DRAFT, STATUS_PUBLISHED } from "@/lib/ui-strings";

type Notice = { kind: "success" | "error"; text: string } | null;
type DragState = {
  mode: "move" | "resize";
  startX: number;
  startY: number;
  initialX: number;
  initialY: number;
  initialWidth: number;
};
type PendingAction =
  | { kind: "save" }
  | { kind: "review"; versionId: string; decision: "published" | "rejected" }
  | { kind: "approve"; jobId: string }
  | { kind: "publish_rule" };

export const drugInspectionStampQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: FIELD_KEYWORD,
    type: "text",
    placeholder: "版本 / 配置人 / 审核人 / 药检单版本 / 错误",
  },
  {
    key: "status",
    label: COLUMN_STATUS,
    type: "multiSelect",
    options: [
      { label: FILTER_ALL, value: "" },
      { label: "待发布审核", value: "pending_review" },
      { label: STATUS_PUBLISHED, value: "published" },
      { label: "已退回", value: "rejected" },
      { label: "已替代", value: "superseded" },
      { label: "副本待处理", value: "queued" },
      { label: "副本生成中", value: "processing" },
      { label: "副本可用", value: "succeeded" },
      { label: "副本失败", value: "failed" },
      { label: "待超限审批", value: "oversize_review" },
    ],
  },
];
export const drugInspectionStampCoreQueryFieldKeys = ["keyword", "status"];

export function DrugInspectionStampPage() {
  const previewRef = React.useRef<HTMLDivElement>(null);
  const [file, setFile] = React.useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = React.useState("");
  const [relativeX, setRelativeX] = React.useState(0.68);
  const [relativeY, setRelativeY] = React.useState(0.72);
  const [relativeWidth, setRelativeWidth] = React.useState(0.2);
  const [drag, setDrag] = React.useState<DragState | null>(null);
  const [reviewComment, setReviewComment] = React.useState("");
  const [oversizeReasons, setOversizeReasons] = React.useState<Record<string, string>>({});
  const [notice, setNotice] = React.useState<Notice>(null);
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>({
    keyword: "",
    status: "",
  });
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>({
    keyword: "",
    status: "",
  });
  const [pendingAction, setPendingAction] = React.useState<PendingAction | null>(null);
  const [processingRuleScope, setProcessingRuleScope] = React.useState<
    "future_only" | "reprocess_current"
  >("future_only");
  const versions = useDrugInspectionStampVersionsQuery();
  const copyJobs = useDrugInspectionCopyJobsQuery();
  const createStamp = useCreateDrugInspectionStampMutation();
  const reviewStamp = useReviewDrugInspectionStampMutation();
  const approveOversize = useApproveDrugInspectionCopyOversizeMutation();
  const processingRules = useDrugInspectionProcessingRulesQuery();
  const publishProcessingRule = usePublishDrugInspectionProcessingRuleMutation();
  const filteredVersions = React.useMemo(
    () =>
      (versions.data ?? []).filter((row) =>
        matchesQuery(
          appliedQuery,
          row.status,
          `v${row.version_number} ${row.configured_by} ${row.reviewed_by ?? ""}`,
        ),
      ),
    [appliedQuery, versions.data],
  );
  const filteredCopyJobs = React.useMemo(
    () =>
      (copyJobs.data ?? []).filter((row) =>
        matchesQuery(
          appliedQuery,
          row.status,
          `${row.report_version_id} ${row.last_error ?? ""}`,
        ),
      ),
    [appliedQuery, copyJobs.data],
  );

  React.useEffect(() => {
    if (!file) {
      setPreviewUrl("");
      return;
    }
    const url = URL.createObjectURL(file);
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [file]);

  React.useEffect(() => {
    if (!drag) return;
    const move = (event: PointerEvent) => {
      const rect = previewRef.current?.getBoundingClientRect();
      if (!rect) return;
      const deltaX = (event.clientX - drag.startX) / rect.width;
      const deltaY = (event.clientY - drag.startY) / rect.height;
      if (drag.mode === "move") {
        setRelativeX(clamp(drag.initialX + deltaX, 0, 1 - relativeWidth));
        setRelativeY(clamp(drag.initialY + deltaY, 0, 0.95));
      } else {
        setRelativeWidth(clamp(drag.initialWidth + deltaX, 0.08, 1 - relativeX));
      }
    };
    const stop = () => setDrag(null);
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop, { once: true });
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
  }, [drag, relativeWidth, relativeX]);

  const versionColumns = React.useMemo<DataGridColumn<DrugInspectionStampVersion>[]>(() => [
    {
      key: "version_number",
      header: COLUMN_VERSION,
      width: 90,
      render: (row) => `v${row.version_number}`,
    },
    {
      key: "status",
      header: COLUMN_STATUS,
      width: 130,
      render: (row) => (
        <StatusBadge
          size="sm"
          status={
            row.status === "published"
              ? "completed"
              : row.status === "pending_review"
                ? "pending"
                : row.status === "rejected"
                  ? "unqualified"
                  : "in_progress"
          }
          label={stampStatusLabel(row.status)}
        />
      ),
    },
    {
      key: "placement",
      header: "相对位置 / 宽度",
      width: 250,
      render: (row) => `${percent(row.relative_x)}, ${percent(row.relative_y)} / ${percent(row.relative_width)}`,
    },
    {
      key: "configured_by",
      header: "配置人",
      width: 260,
      render: (row) => <span className="font-mono text-xs">{row.configured_by}</span>,
    },
    {
      key: "reviewed_by",
      header: "审核人",
      width: 260,
      render: (row) => row.reviewed_by
        ? <span className="font-mono text-xs">{row.reviewed_by}</span>
        : "—",
    },
    {
      key: "created_at",
      header: COLUMN_CREATED_AT,
      width: 180,
      render: (row) => formatDateTime(row.created_at),
    },
    {
      key: "actions",
      header: "操作",
      width: 190,
      render: (row) => row.status === "pending_review" ? (
        <div className="flex gap-2">
          <Button
            size="sm"
            type="button"
            onClick={() =>
              setPendingAction({
                kind: "review",
                versionId: row.id,
                decision: "published",
              })}
          >
            <Check className="size-4" aria-hidden />
            发布
          </Button>
          <Button
            size="sm"
            type="button"
            variant="outline"
            onClick={() =>
              setPendingAction({
                kind: "review",
                versionId: row.id,
                decision: "rejected",
              })}
          >
            <X className="size-4" aria-hidden />
            退回
          </Button>
        </div>
      ) : "—",
    },
  ], [reviewComment]);

  const jobColumns = React.useMemo<DataGridColumn<DrugInspectionCustomerCopyJob>[]>(() => [
    {
      key: "status",
      header: "副本状态",
      width: 150,
      render: (row) => (
        <StatusBadge
          size="sm"
          status={row.status === "succeeded" ? "completed" : row.status === "failed" ? "unqualified" : "pending"}
          label={copyStatusLabel(row.status)}
        />
      ),
    },
    {
      key: "report_version_id",
      header: "药检单版本 ID",
      width: 300,
      render: (row) => <span className="font-mono text-xs">{row.report_version_id}</span>,
    },
    {
      key: "attempt_count",
      header: "处理次数",
      width: 110,
    },
    {
      key: "last_error",
      header: "最近错误",
      width: 300,
      render: (row) => row.last_error || "—",
    },
    {
      key: "updated_at",
      header: COLUMN_UPDATED_AT,
      width: 180,
      render: (row) => formatDateTime(row.updated_at),
    },
    {
      key: "actions",
      header: "超限审批",
      width: 340,
      render: (row) => row.status === "oversize_review" ? (
        <div className="flex gap-2">
          <Input
            aria-label={`副本 ${row.id} 超限批准原因`}
            value={oversizeReasons[row.id] ?? ""}
            onChange={(event) => setOversizeReasons((current) => ({
              ...current,
              [row.id]: event.target.value,
            }))}
            placeholder="填写超过 50MB 的批准原因"
          />
          <Button
            size="sm"
            type="button"
            onClick={() => setPendingAction({ kind: "approve", jobId: row.id })}
          >
            批准
          </Button>
        </div>
      ) : "—",
    },
  ], [oversizeReasons]);

  async function save() {
    if (!file) {
      setNotice({ kind: "error", text: "请选择带透明通道的 PNG 图章" });
      return;
    }
    if (file.type !== "image/png" || file.size > 5 * 1024 * 1024) {
      setNotice({ kind: "error", text: "图章必须是 5MB 以内的透明 PNG" });
      return;
    }
    try {
      const version = await createStamp.mutateAsync({
        file,
        relativeX,
        relativeY,
        relativeWidth,
      });
      setNotice({ kind: "success", text: `图章 v${version.version_number} 已提交另一名质量负责人审核` });
      setFile(null);
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "保存图章失败" });
    }
  }

  async function review(versionId: string, decision: "published" | "rejected") {
    if (decision === "rejected" && !reviewComment.trim()) {
      setNotice({ kind: "error", text: "退回图章必须填写审核意见" });
      return;
    }
    try {
      await reviewStamp.mutateAsync({
        versionId,
        decision,
        comment: reviewComment.trim() || undefined,
      });
      setReviewComment("");
      setNotice({ kind: "success", text: decision === "published" ? "图章版本已发布" : "图章版本已退回" });
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "审核图章失败" });
    }
  }

  async function approve(jobId: string) {
    const reason = oversizeReasons[jobId]?.trim() ?? "";
    if (!reason) {
      setNotice({ kind: "error", text: "批准超过 50MB 的客户副本必须填写原因" });
      return;
    }
    try {
      await approveOversize.mutateAsync({ jobId, reason });
      setNotice({ kind: "success", text: "超限客户副本已批准并可供客户查询" });
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "批准超限副本失败" });
    }
  }

  async function publishRule() {
    try {
      const rule = await publishProcessingRule.mutateAsync(processingRuleScope);
      setNotice({
        kind: "success",
        text: processingRuleScope === "reprocess_current"
          ? `处理规则 v${rule.version_number} 已发布，已创建 ${rule.reprocess_job_count} 个当前报告重处理任务`
          : `处理规则 v${rule.version_number} 已发布，仅用于之后确认的药检单`,
      });
    } catch (error) {
      setNotice({
        kind: "error",
        text: error instanceof Error ? error.message : "发布处理规则失败",
      });
    }
  }

  async function confirmPendingAction() {
    const action = pendingAction;
    if (!action) return;
    setPendingAction(null);
    if (action.kind === "save") {
      await save();
    } else if (action.kind === "review") {
      await review(action.versionId, action.decision);
    } else if (action.kind === "approve") {
      await approve(action.jobId);
    } else {
      await publishRule();
    }
  }

  function startDrag(event: React.PointerEvent, mode: DragState["mode"]) {
    event.preventDefault();
    setDrag({
      mode,
      startX: event.clientX,
      startY: event.clientY,
      initialX: relativeX,
      initialY: relativeY,
      initialWidth: relativeWidth,
    });
  }

  return (
    <ListPageTemplate
      notice={notice}
      queryFields={drugInspectionStampQueryFields}
      coreQueryFieldKeys={drugInspectionStampCoreQueryFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={setDraftQuery}
      onQuery={() => setAppliedQuery(draftQuery)}
      onReset={() => {
        const empty = { keyword: "", status: "" };
        setDraftQuery(empty);
        setAppliedQuery(empty);
      }}
      dialogs={
        <Dialog
          open={pendingAction !== null}
          onOpenChange={(open) => !open && setPendingAction(null)}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{pendingActionTitle(pendingAction)}</DialogTitle>
              <DialogDescription>
                {pendingActionDescription(pendingAction)}
              </DialogDescription>
            </DialogHeader>
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setPendingAction(null)}
              >
                取消
              </Button>
              <Button
                type="button"
                disabled={isPendingActionBusy(
                  pendingAction,
                  createStamp.isPending,
                  reviewStamp.isPending,
                  approveOversize.isPending,
                  publishProcessingRule.isPending,
                )}
                onClick={() => void confirmPendingAction()}
              >
                {pendingActionConfirmLabel(pendingAction)}
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      }
    >
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <Card className="flex flex-1 flex-col min-h-0 overflow-hidden">
          <CardContent className="flex flex-1 flex-col min-h-0 p-5">
            <div
              ref={previewRef}
              className="relative mx-auto aspect-[210/297] w-full max-w-[620px] overflow-hidden rounded-sm border bg-white shadow-sm"
              aria-label="药检单图章位置设计器"
            >
              <div className="space-y-4 p-[8%] text-slate-300">
                <div className="mx-auto h-5 w-2/5 rounded bg-slate-200" />
                {Array.from({ length: 18 }, (_, index) => (
                  <div key={index} className="h-2 rounded bg-slate-100" />
                ))}
              </div>
              {previewUrl && (
                <div
                  className="absolute cursor-move touch-none select-none border border-dashed border-primary"
                  style={{
                    left: `${relativeX * 100}%`,
                    top: `${relativeY * 100}%`,
                    width: `${relativeWidth * 100}%`,
                  }}
                  onPointerDown={(event) => startDrag(event, "move")}
                >
                  <img src={previewUrl} alt="待发布的透明 PNG 图章" className="pointer-events-none block h-auto w-full" />
                  <button
                    type="button"
                    aria-label="拖动缩放图章"
                    className="absolute -bottom-2 -right-2 size-5 cursor-se-resize rounded-full border-2 border-white bg-primary shadow"
                    onPointerDown={(event) => {
                      event.stopPropagation();
                      startDrag(event, "resize");
                    }}
                  />
                </div>
              )}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="grid gap-4 p-5">
            <label className="grid gap-2 text-sm font-medium">
              透明 PNG 图章
              <Input
                type="file"
                accept="image/png"
                onChange={(event) => setFile(event.target.files?.[0] ?? null)}
              />
            </label>
            <div className="grid grid-cols-3 gap-2 text-sm">
              <Metric label="横向" value={percent(relativeX)} />
              <Metric label="纵向" value={percent(relativeY)} />
              <Metric label="宽度" value={percent(relativeWidth)} />
            </div>
            <p className="text-sm text-muted-foreground">
              位置和宽度按页面比例保存，并应用到副本的全部页面。权威原件不会被覆盖。
            </p>
            <Button
              type="button"
              disabled={!file || createStamp.isPending}
              onClick={() => setPendingAction({ kind: "save" })}
            >
              <Send className="size-4" aria-hidden />
              上传并提交审核
            </Button>
            <label className="grid gap-2 text-sm font-medium">
              审核意见
              <Input
                value={reviewComment}
                onChange={(event) => setReviewComment(event.target.value)}
                placeholder="退回时必填；发布时可填写确认说明"
              />
            </label>
            <div className="flex items-center gap-2 rounded-md bg-muted/40 p-3 text-sm">
              <FileImage className="size-4" aria-hidden />
              发布后仅用于随后确认并生成的客户分发副本。
            </div>
          </CardContent>
        </Card>
      </div>
      <Card className="flex flex-1 flex-col min-h-0 overflow-hidden">
        <CardContent className="flex flex-1 flex-col min-h-0 p-5">
          <h2 className="mb-3 text-base font-semibold">图章版本记录</h2>
          <DataGrid
            storageKey="m-di.stamp-versions"
            columns={versionColumns}
            data={filteredVersions}
            rowKey={(row) => row.id}
            caption={`共 ${filteredVersions.length} 个图章版本`}
            emptyTitle={versions.isError ? "读取图章版本失败" : "暂无图章版本"}
            emptyDescription={versions.isError ? versions.error.message : "上传第一个透明 PNG 图章"}
            tableClassName="min-w-[1280px]"
          />
        </CardContent>
      </Card>
      <Card>
        <CardContent className="grid gap-4 p-5 md:grid-cols-[minmax(0,1fr)_320px_auto] md:items-end">
          <div>
            <h2 className="text-base font-semibold">图像处理规则版本</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              新版本不会覆盖历史副本；重处理成功后才原子切换当前客户副本。
            </p>
            <p className="mt-2 text-sm">
              当前版本：
              {processingRules.data?.[0]
                ? `v${processingRules.data[0].version_number} · ${processingRules.data[0].rule_code}`
                : "内置 mdi-image-v1"}
            </p>
          </div>
          <label className="grid gap-2 text-sm font-medium">
            处理规则应用范围
            <select
              className="h-10 rounded-md border border-input bg-background px-3 text-sm"
              value={processingRuleScope}
              onChange={(event) => setProcessingRuleScope(
                event.target.value as "future_only" | "reprocess_current",
              )}
            >
              <option value="future_only">仅用于之后确认的药检单</option>
              <option value="reprocess_current">重新处理当前有效药检单</option>
            </select>
          </label>
          <Button
            type="button"
            disabled={publishProcessingRule.isPending}
            onClick={() => setPendingAction({ kind: "publish_rule" })}
          >
            {publishProcessingRule.isPending ? "发布中…" : "发布处理规则版本"}
          </Button>
        </CardContent>
      </Card>
      <Card className="flex flex-1 flex-col min-h-0 overflow-hidden">
        <CardContent className="flex flex-1 flex-col min-h-0 p-5">
          <h2 className="mb-3 text-base font-semibold">客户副本后台任务</h2>
          <DataGrid
            storageKey="m-di.customer-copy-jobs"
            columns={jobColumns}
            data={filteredCopyJobs}
            rowKey={(row) => row.id}
            caption={`共 ${filteredCopyJobs.length} 个客户副本任务`}
            emptyTitle={copyJobs.isError ? "读取副本任务失败" : "暂无客户副本任务"}
            emptyDescription={copyJobs.isError ? copyJobs.error.message : "药检单确认后会自动进入后台处理"}
            tableClassName="min-w-[1420px]"
          />
        </CardContent>
      </Card>
    </ListPageTemplate>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="rounded-md border p-2 text-center"><div className="text-muted-foreground">{label}</div><div className="mt-1 font-mono">{value}</div></div>;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function percent(value: number) {
  return `${(value * 100).toFixed(1)}%`;
}

function formatDateTime(value?: string | null) {
  return value ? new Date(value).toLocaleString("zh-CN", { hour12: false }) : "—";
}

function stampStatusLabel(status: string) {
  return {
    draft: STATUS_DRAFT,
    pending_review: "待发布审核",
    published: STATUS_PUBLISHED,
    rejected: "已退回",
    superseded: "已替代",
  }[status] ?? status;
}

function copyStatusLabel(status: string) {
  return {
    queued: "排队中",
    processing: "生成中",
    succeeded: "可用",
    failed: "生成失败",
    oversize_review: "待超限审批",
  }[status] ?? status;
}

function matchesQuery(
  query: QueryPanelValue,
  status: string,
  searchableText: string,
) {
  const statusFilter = typeof query.status === "string" ? query.status : "";
  const keyword =
    typeof query.keyword === "string"
      ? query.keyword.trim().toLocaleLowerCase()
      : "";
  return (
    (!statusFilter || status === statusFilter) &&
    (!keyword || searchableText.toLocaleLowerCase().includes(keyword))
  );
}

function pendingActionTitle(action: PendingAction | null) {
  if (!action) return "确认操作";
  if (action.kind === "save") return "确认上传图章";
  if (action.kind === "approve") return "确认批准超限副本";
  if (action.kind === "publish_rule") return "确认发布处理规则";
  return action.decision === "published" ? "确认发布图章" : "确认退回图章";
}

function pendingActionDescription(action: PendingAction | null) {
  if (!action) return "";
  if (action.kind === "save") {
    return "图章将作为新版本上传，并提交另一名质量负责人审核。";
  }
  if (action.kind === "approve") {
    return "批准后，超过 50MB 的客户副本将可供授权客户查询。";
  }
  if (action.kind === "publish_rule") {
    return "发布后将按所选范围影响之后生成或重新处理的客户副本。";
  }
  return action.decision === "published"
    ? "发布后，此版本会用于随后生成的客户副本。"
    : "退回后，配置人员需要根据审核意见创建新版本。";
}

function pendingActionConfirmLabel(action: PendingAction | null) {
  if (!action) return "确认";
  if (action.kind === "save") return "确认上传并提交审核";
  if (action.kind === "approve") return "确认批准";
  if (action.kind === "publish_rule") return "确认发布处理规则版本";
  return action.decision === "published" ? "确认发布" : "确认退回";
}

function isPendingActionBusy(
  action: PendingAction | null,
  saving: boolean,
  reviewing: boolean,
  approving: boolean,
  publishingRule: boolean,
) {
  if (!action) return false;
  if (action.kind === "save") return saving;
  if (action.kind === "review") return reviewing;
  if (action.kind === "approve") return approving;
  return publishingRule;
}
