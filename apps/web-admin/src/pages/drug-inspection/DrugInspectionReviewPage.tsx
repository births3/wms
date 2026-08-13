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
  PageHeader,
  QueryPanel,
  StatusBadge,
  cn,
  type DataGridColumn,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { CheckCircle2, ExternalLink, RotateCcw } from "lucide-react";

import {
  getAttachmentDownloadUrl,
  type DrugInspectionReviewQueueEntry,
  useDrugInspectionReviewQueueQuery,
  useDrugInspectionVersionsQuery,
  useReviewDrugInspectionMutation,
} from "@/features/drug-inspection/document-queries";
import { COLUMN_BATCH_NO, COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_VERSION, FIELD_KEYWORD } from "@/lib/ui-strings";
import { DrugInspectionRequirementRulesCard } from "./DrugInspectionRequirementRulesCard";

type Notice = { kind: "success" | "error"; text: string } | null;

export const drugInspectionReviewQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: FIELD_KEYWORD,
    type: "text",
    placeholder: "报告编号 / 商品 / 批号 / 上传人",
  },
];
export const drugInspectionReviewCoreQueryFieldKeys = ["keyword"];

export function DrugInspectionReviewPage() {
  const queue = useDrugInspectionReviewQueueQuery();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>({ keyword: "" });
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>({ keyword: "" });
  const [selected, setSelected] =
    React.useState<DrugInspectionReviewQueueEntry | null>(null);
  const [comment, setComment] = React.useState("");
  const [notice, setNotice] = React.useState<Notice>(null);
  const versions = useDrugInspectionVersionsQuery(selected?.version.report_id ?? null);
  const review = useReviewDrugInspectionMutation();
  const filteredQueue = React.useMemo(() => {
    const keyword =
      typeof appliedQuery.keyword === "string"
        ? appliedQuery.keyword.trim().toLocaleLowerCase()
        : "";
    if (!keyword) return queue.data ?? [];
    return (queue.data ?? []).filter((row) =>
      [
        row.version.report_no,
        row.product_code,
        row.product_name,
        row.batch_no,
        row.uploader_name,
      ]
        .join(" ")
        .toLocaleLowerCase()
        .includes(keyword),
    );
  }, [queue.data, appliedQuery]);
  const columns = React.useMemo<
    DataGridColumn<DrugInspectionReviewQueueEntry>[]
  >(
    () => [
      textColumn("report", "报告编号", (row) => row.version.report_no, 180),
      textColumn("product", "商品", (row) => `${row.product_code} ${row.product_name}`, 220),
      textColumn("batch", COLUMN_BATCH_NO, (row) => row.batch_no, 150),
      textColumn("uploader", "上传人", (row) => row.uploader_name, 150),
      textColumn(
        "submitted",
        "提交时间",
        (row) => formatDateTime(row.version.submitted_at),
        180,
      ),
      {
        key: "created_at",
        header: COLUMN_CREATED_AT,
        width: 180,
        render: (row) => formatDateTime(row.version.created_at),
      },
      {
        key: "status",
        header: COLUMN_STATUS,
        width: 130,
        render: () => (
          <StatusBadge status="pending" label="待确认" size="sm" />
        ),
      },
      {
        key: "action",
        header: "操作",
        width: 120,
        render: (row) => (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => {
              setSelected(row);
              setComment("");
              setNotice(null);
            }}
          >
            审核
          </Button>
        ),
      },
    ],
    [],
  );

  async function openOriginal() {
    if (!selected) return;
    try {
      const url = await getAttachmentDownloadUrl(
        selected.version.original_file_id,
      );
      window.open(url, "_blank", "noopener,noreferrer");
    } catch (error) {
      setNotice({
        kind: "error",
        text: error instanceof Error ? error.message : "打开权威原件失败",
      });
    }
  }

  async function decide(decision: "confirmed" | "rejected") {
    if (!selected) return;
    if (decision === "rejected" && !comment.trim()) {
      setNotice({ kind: "error", text: "退回修改必须填写审核意见" });
      return;
    }
    try {
      await review.mutateAsync({
        versionId: selected.version.id,
        decision,
        comment: comment.trim() || undefined,
      });
      setNotice({
        kind: "success",
        text: decision === "confirmed" ? "药检单已确认" : "药检单已退回修改",
      });
      setSelected(null);
    } catch (error) {
      setNotice({
        kind: "error",
        text: error instanceof Error ? error.message : "审核药检单失败",
      });
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader />
      {notice && <NoticeBox notice={notice} />}
      <QueryPanel
        fields={drugInspectionReviewQueryFields}
        defaultVisibleFieldKeys={drugInspectionReviewCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => setAppliedQuery(draftQuery)}
        onReset={() => {
          setDraftQuery({ keyword: "" });
          setAppliedQuery({ keyword: "" });
        }}
      />
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m-di.review-queue"
            columns={columns}
            data={filteredQueue}
            rowKey={(row) => row.version.id}
            caption={queue.isPending ? "加载待审核药检单..." : `待审核 ${queue.data?.length ?? 0} 份`}
            emptyTitle={queue.isError ? "读取审核队列失败" : "没有待审核药检单"}
            emptyDescription={queue.isError ? queue.error.message : "新提交的药检单会出现在这里"}
            tableClassName="min-w-[1180px]"
          />
        </CardContent>
      </Card>
      <DrugInspectionRequirementRulesCard />

      {selected && (
        <Dialog open onOpenChange={(open) => !open && setSelected(null)}>
          <DialogContent className="max-h-[92vh] overflow-y-auto sm:max-w-4xl">
            <DialogHeader>
              <DialogTitle>审核药检单 · {selected.version.report_no}</DialogTitle>
              <DialogDescription>
                {selected.product_code} {selected.product_name} · 批号 {selected.batch_no}
              </DialogDescription>
            </DialogHeader>
            {notice && <NoticeBox notice={notice} />}
            <div className="grid gap-3 rounded-md border p-4 md:grid-cols-2">
              <Info label={COLUMN_VERSION} value={`v${selected.version.version_number}`} />
              <Info label="上传人" value={selected.uploader_name} />
              <Info
                label="图像处理"
                value={processingModeLabel(selected.version.processing_mode)}
              />
              <Info
                label="检验结论"
                value={selected.version.qualified ? "合格" : "不合格"}
              />
              <Info
                label="原件哈希"
                value={selected.version.original_file_hash}
                mono
              />
              <Info
                label="提交时间"
                value={formatDateTime(selected.version.submitted_at)}
              />
            </div>
            <Button type="button" variant="outline" onClick={() => void openOriginal()}>
              <ExternalLink className="size-4" aria-hidden />
              查看权威原件
            </Button>
            <label className="grid gap-1 text-sm">
              <span>审核意见（退回时必填）</span>
              <Input
                value={comment}
                onChange={(event) => setComment(event.target.value)}
                placeholder="填写退回原因或确认备注"
              />
            </label>
            <div className="rounded-md border p-4">
              <h3 className="font-medium">版本与审核时间线</h3>
              <div className="mt-3 grid gap-3">
                {(versions.data ?? []).map((version) => (
                  <div key={version.id} className="rounded-md bg-muted/40 p-3 text-sm">
                    <div className="flex items-center justify-between gap-3">
                      <span className="font-medium">
                        v{version.version_number} · {statusLabel(version.status)}
                      </span>
                      <span className="text-muted-foreground">
                        {formatDateTime(version.updated_at)}
                      </span>
                    </div>
                    <p className="mt-1 break-all font-mono text-xs text-muted-foreground">
                      {version.original_file_hash}
                    </p>
                    {version.modification_reason && (
                      <p className="mt-1">修改原因：{version.modification_reason}</p>
                    )}
                    {version.review_comment && (
                      <p className="mt-1">审核意见：{version.review_comment}</p>
                    )}
                  </div>
                ))}
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                disabled={review.isPending}
                onClick={() => void decide("rejected")}
              >
                <RotateCcw className="size-4" aria-hidden />
                退回修改
              </Button>
              <Button
                type="button"
                disabled={review.isPending}
                onClick={() => void decide("confirmed")}
              >
                <CheckCircle2 className="size-4" aria-hidden />
                确认通过
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </section>
  );
}

function textColumn(
  key: string,
  header: string,
  value: (row: DrugInspectionReviewQueueEntry) => string,
  width: number,
): DataGridColumn<DrugInspectionReviewQueueEntry> {
  return {
    key,
    header,
    width,
    sortable: true,
    sortValue: value,
    filterValue: value,
    filter: { type: "text" },
    render: (row) => value(row),
  };
}

function NoticeBox({ notice }: { notice: NonNullable<Notice> }) {
  return (
    <div
      role={notice.kind === "error" ? "alert" : "status"}
      className={cn(
        "rounded-md border px-3 py-2 text-sm",
        notice.kind === "error"
          ? "border-destructive/30 bg-destructive/10 text-destructive"
          : "border-wms-success/30 bg-wms-success/10 text-wms-success",
      )}
    >
      {notice.text}
    </div>
  );
}

function Info({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={cn("mt-1 break-all", mono && "font-mono text-xs")}>{value}</div>
    </div>
  );
}

function formatDateTime(value?: string | null) {
  return value
    ? new Intl.DateTimeFormat("zh-CN", {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone: "Asia/Shanghai",
    }).format(new Date(value))
    : "-";
}

function processingModeLabel(value: string) {
  return (
    {
      none: "不处理",
      color_enhance: "原色增强",
      black_white_enhance: "黑白增强",
    }[value] ?? value
  );
}

function statusLabel(value: string) {
  return (
    {
      draft: "草稿/已退回",
      pending_confirmation: "待确认",
      confirmed: "已确认",
      superseded: "已被更正替代",
    }[value] ?? value
  );
}
