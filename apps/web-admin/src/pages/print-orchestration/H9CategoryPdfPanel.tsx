import * as React from "react";
import {
  Button,
  DataGrid,
  StatusBadge,
  type DataGridColumn,
  type DataGridRefreshAction,
} from "@wms/ui";
import { Download, FileCheck2, Printer } from "lucide-react";

import {
  useCategoryPdfsQuery,
  useDownloadCategoryPdfsMutation,
  usePrepareCategoryPdfsMutation,
  type CategoryPdfOutput,
  type PrintDocumentCategory,
  type PrintSuiteInstance,
} from "@/features/print-orchestration/print-orchestration-queries";
import { formatDateTime } from "@/lib/format";

interface H9CategoryPdfPanelProps {
  instances: PrintSuiteInstance[];
  categories: PrintDocumentCategory[];
  canRead: boolean;
  canPrepare: boolean;
  canDownload: boolean;
  canEmergencyPrint: boolean;
  onNotice: (message: string) => void;
}

export function H9CategoryPdfPanel({
  instances,
  categories,
  canRead,
  canPrepare,
  canDownload,
  canEmergencyPrint,
  onNotice,
}: H9CategoryPdfPanelProps) {
  const [instanceId, setInstanceId] = React.useState("");
  const [selectedPdfIds, setSelectedPdfIds] = React.useState<string[]>([]);
  const preparationKeys = React.useRef(new Map<string, string>());
  const categoryLabels = React.useMemo(
    () => new Map(categories.map((category) => [category.item_code, category.item_name])),
    [categories],
  );
  const query = useCategoryPdfsQuery(canRead && instanceId ? instanceId : null);
  const prepare = usePrepareCategoryPdfsMutation();
  const download = useDownloadCategoryPdfsMutation(false);
  const emergencyPrint = useDownloadCategoryPdfsMutation(true);
  const outputs = query.data?.data ?? [];
  const selectedInstance = instances.find((instance) => instance.id === instanceId) ?? null;
  const allReady = outputs.length > 0
    && outputs.every((output) => output.processing_status === "ready");

  React.useEffect(() => {
    if (instances.some((instance) => instance.id === instanceId)) return;
    setInstanceId(instances.find((instance) => instance.status === "waiting_documents")?.id
      ?? instances[0]?.id
      ?? "");
    setSelectedPdfIds([]);
  }, [instanceId, instances]);

  React.useEffect(() => {
    const retryKey = query.data?.retry_idempotency_key;
    if (instanceId && retryKey) preparationKeys.current.set(instanceId, retryKey);
  }, [instanceId, query.data?.retry_idempotency_key]);

  async function preparePdfs() {
    if (!instanceId) return;
    const requestKey = preparationKeys.current.get(instanceId)
      ?? `web-h9-category-pdf-${globalThis.crypto.randomUUID()}`;
    preparationKeys.current.set(instanceId, requestKey);
    const result = await prepare.mutateAsync({ instanceId, idempotencyKey: requestKey });
    setSelectedPdfIds([]);
    onNotice(
      result.status === "completed"
        ? `分类 PDF 已生成：${result.outputs.length} 个分类产物，组套实例已进入待打印`
        : "分类 PDF 生成失败：实例保持等待单据，可使用同一幂等键重试",
    );
  }

  async function receivePdfs(emergency: boolean) {
    if (!instanceId) return;
    const mutation = emergency ? emergencyPrint : download;
    const blob = await mutation.mutateAsync({
      instanceId,
      categoryPdfIds: selectedPdfIds,
    });
    const url = URL.createObjectURL(blob);
    if (emergency) {
      window.open(url, "_blank", "noopener,noreferrer");
      onNotice(`已打开${selectedPdfIds.length || outputs.length} 个分类的应急打印 PDF`);
    } else {
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = `${selectedInstance?.delivery_note_no ?? "h9"}-分类PDF.pdf`;
      anchor.click();
      onNotice(`已下载${selectedPdfIds.length || outputs.length} 个分类的临时合并 PDF`);
    }
    globalThis.setTimeout(() => URL.revokeObjectURL(url), 60_000);
  }

  const columns = React.useMemo(
    () => categoryPdfColumns(categoryLabels),
    [categoryLabels],
  );
  const busy = prepare.isPending || download.isPending || emergencyPrint.isPending;
  const error = query.error ?? prepare.error ?? download.error ?? emergencyPrint.error;
  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新所选组套实例的分类 PDF 元数据",
    disabled: query.isFetching || !instanceId,
    onClick: () => void query.refetch(),
  };

  if (!canRead) {
    return <p className="text-sm text-muted-foreground">当前账号没有分类 PDF 查看权限。</p>;
  }

  return (
    <section className="space-y-3" aria-label="分类 PDF 生成与留存">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <label className="grid min-w-80 gap-1 text-sm font-medium">
          <span>组套实例</span>
          <select
            className="h-10 rounded-md border border-input bg-background px-3 text-sm"
            value={instanceId}
            onChange={(event) => {
              setInstanceId(event.target.value);
              setSelectedPdfIds([]);
            }}
          >
            <option value="">请选择组套实例</option>
            {instances.map((instance) => (
              <option key={instance.id} value={instance.id}>
                {instance.delivery_note_no} · V{instance.suite_version_no} · {instanceStatusLabel(instance.status)}
              </option>
            ))}
          </select>
        </label>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="outline"
            disabled={!canPrepare || !instanceId || allReady || prepare.isPending}
            onClick={() => void preparePdfs().catch(() => undefined)}
          >
            <FileCheck2 className="mr-2 size-4" aria-hidden />
            {query.data?.preparation_status === "failed" ? "重试生成分类 PDF" : "生成分类 PDF"}
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={!canDownload || !allReady || busy}
            onClick={() => void receivePdfs(false).catch(() => undefined)}
          >
            <Download className="mr-2 size-4" aria-hidden />
            {selectedPdfIds.length ? "下载所选分类" : "下载全部分类"}
          </Button>
          <Button
            type="button"
            disabled={!canEmergencyPrint || !allReady || busy}
            onClick={() => void receivePdfs(true).catch(() => undefined)}
          >
            <Printer className="mr-2 size-4" aria-hidden />
            {selectedPdfIds.length ? "应急打印所选" : "应急打印全部"}
          </Button>
        </div>
      </div>
      <p className="text-sm text-muted-foreground">
        服务端按分类生成 PDF；随货同行单按 GSP 五年策略归档，已有权威发票和药检单仅保留引用或短期缓存。
        多分类下载只临时合并，不新增完整组套文件。
      </p>
      {error && (
        <p className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">
          {error.message}
        </p>
      )}
      <DataGrid
        columns={columns}
        data={outputs}
        rowKey={(row) => row.id}
        storageKey="h9-category-pdfs"
        emptyTitle={instanceId ? "尚未生成分类 PDF" : "请选择组套实例"}
        emptyDescription={instanceId
          ? "源单据就绪后生成；失败不会创建可执行打印任务"
          : "选择实例后查看源版本、模板版本、哈希和留存策略"}
        caption={query.isPending ? "加载分类 PDF..." : undefined}
        refreshAction={refreshAction}
        selectedRowKeys={selectedPdfIds}
        onSelectedRowKeysChange={setSelectedPdfIds}
        selectable
        tableClassName="min-w-[1540px]"
      />
    </section>
  );
}

function categoryPdfColumns(
  categoryLabels: Map<string, string>,
): DataGridColumn<CategoryPdfOutput>[] {
  return [
    {
      key: "sort_order",
      header: "组套顺序",
      width: 90,
      mono: true,
      render: (row) => row.sort_order,
    },
    {
      key: "category_code",
      header: "单据分类",
      width: 150,
      render: (row) => categoryLabels.get(row.category_code) ?? row.category_code,
    },
    {
      key: "processing_status",
      header: "处理状态",
      width: 110,
      render: (row) => (
        <StatusBadge
          status={row.processing_status === "ready"
            ? "completed"
            : row.processing_status === "failed"
              ? "unqualified"
              : "pending"}
          label={processingStatusLabel(row.processing_status)}
          size="sm"
        />
      ),
    },
    {
      key: "source_mode",
      header: "来源模式",
      width: 110,
      render: (row) => row.source_mode === "rendered" ? "服务端渲染" : "权威外部 PDF",
    },
    {
      key: "source_fact",
      header: "源数据版本 / 权威文件",
      width: 300,
      mono: true,
      render: (row) => row.source_mode === "rendered"
        ? shortHash(row.source_data_version)
        : row.source_file_bindings.map((binding) => `${binding.file_ref} · V${binding.file_version}`).join("；"),
    },
    {
      key: "template_version_id",
      header: "模板版本 ID",
      width: 270,
      mono: true,
      render: (row) => row.template_version_id ?? "不适用（权威外部 PDF）",
    },
    {
      key: "content_hash",
      header: "内容 SHA-256",
      width: 180,
      mono: true,
      render: (row) => shortHash(row.content_hash),
    },
    {
      key: "retention_policy",
      header: "留存策略",
      width: 140,
      render: (row) => row.retention_policy === "gsp_5_year"
        ? "GSP 五年归档"
        : `短期缓存${row.cache_expires_at ? `（至 ${formatDateTime(row.cache_expires_at)}）` : ""}`,
    },
    {
      key: "attempt_count",
      header: "处理次数",
      width: 100,
      mono: true,
      render: (row) => row.attempt_count,
    },
    {
      key: "processed_at",
      header: "处理时间",
      width: 170,
      render: (row) => formatDateTime(row.processed_at),
    },
    {
      key: "created_at",
      header: "创建时间",
      width: 170,
      defaultHidden: true,
      render: (row) => formatDateTime(row.created_at),
    },
    {
      key: "failure_reason",
      header: "失败原因",
      width: 240,
      defaultHidden: true,
      render: (row) => row.failure_reason ?? "-",
    },
  ];
}

function processingStatusLabel(status: string) {
  return status === "ready"
    ? "已就绪"
    : status === "failed"
      ? "生成失败"
      : status === "processing"
        ? "处理中"
        : "待处理";
}

function instanceStatusLabel(status: string) {
  return status === "queued" ? "待打印" : status === "cancelled" ? "已取消" : "等待分类 PDF";
}

function shortHash(value: string | null | undefined) {
  return value ? `${value.slice(0, 12)}…${value.slice(-8)}` : "-";
}
