import * as React from "react";
import {
  Card,
  CardContent,
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { ExternalLink, FileJson, RefreshCw } from "lucide-react";

import {
  useAuditEventsQuery,
  type AuditEventRow,
} from "@/features/audit/audit-queries";
import { queryRange } from "@/lib/query-value";
import {
  BUTTON_REFRESH,
  COLUMN_BATCH_NO,
  COLUMN_CREATED_AT,
  COLUMN_PRODUCT_CODE,
} from "@/lib/ui-strings";
import { usePageQueryState } from "@/lib/use-page-query-state";

const h2AuditQueryFields: QueryPanelField[] = [
  {
    key: "resourceType",
    label: "对象类型",
    type: "select",
    options: [
      { label: "入库单", value: "receiving_order" },
      { label: "托盘", value: "pallet" },
      { label: "批号调整", value: "batch_adjustment" },
      { label: "出库单", value: "shipping_order" },
      { label: "登录会话", value: "auth_session" },
    ],
  },
  {
    key: "actorId",
    label: "操作人",
    type: "text",
    placeholder: "用户 ID",
  },
  {
    key: "action",
    label: "动作类型",
    type: "text",
    placeholder: "如 receive / update",
  },
  {
    key: "resourceId",
    label: "关联资源",
    type: "text",
    placeholder: "单据号或资源 ID",
  },
  {
    key: "productCode",
    label: COLUMN_PRODUCT_CODE,
    type: "text",
    placeholder: COLUMN_PRODUCT_CODE,
  },
  {
    key: "batchNo",
    label: COLUMN_BATCH_NO,
    type: "text",
    placeholder: COLUMN_BATCH_NO,
  },
  {
    key: "occurredAt",
    label: "发生时间",
    type: "dateRange",
  },
];
const h2AuditCoreQueryFieldKeys = ["resourceType", "occurredAt"];

const h2AuditColumns: DataGridColumn<AuditEventRow>[] = [
  {
    key: "createdAt",
    header: COLUMN_CREATED_AT,
    width: 190,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.occurredAt,
    filterValue: (row) => row.occurredAt,
    copyValue: (row) => formatDateTime(row.occurredAt),
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.occurredAt),
  },
  {
    key: "actorName",
    header: "操作者",
    width: 160,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => row.actorName,
    filterValue: (row) => row.actorName,
    copyValue: (row) => row.actorName,
    filter: { type: "text" },
  },
  {
    key: "action",
    header: "动作",
    width: 180,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.action,
    filterValue: (row) => row.action,
    copyValue: (row) => row.action,
    filter: { type: "text" },
  },
  {
    key: "objectLabel",
    header: "对象",
    width: 280,
    minWidth: 200,
    mono: true,
    sortable: true,
    sortValue: (row) => row.objectLabel,
    filterValue: (row) => row.objectLabel,
    copyValue: (row) => row.objectLabel,
    filter: { type: "text" },
  },
  {
    key: "ipAddress",
    header: "IP 地址",
    width: 150,
    minWidth: 120,
    mono: true,
    sortable: true,
    sortValue: (row) => row.ipAddress,
    filterValue: (row) => row.ipAddress,
    copyValue: (row) => row.ipAddress,
    filter: { type: "text" },
  },
  {
    key: "traceId",
    header: "Trace",
    width: 160,
    minWidth: 120,
    mono: true,
    filterValue: (row) => row.traceId,
    copyValue: (row) => row.traceId,
    filter: { type: "text" },
  },
  {
    key: "diffBefore",
    header: "旧值",
    width: 240,
    minWidth: 180,
    mono: true,
    sortable: false,
    filterValue: (row) => row.diffBefore,
    copyValue: (row) => row.diffBefore,
    filter: { type: "text" },
    render: (row) => <span className="block max-w-[220px] truncate" title={row.diffBefore}>{row.diffBefore}</span>,
  },
  {
    key: "diffAfter",
    header: "新值",
    width: 240,
    minWidth: 180,
    mono: true,
    sortable: false,
    filterValue: (row) => row.diffAfter,
    copyValue: (row) => row.diffAfter,
    filter: { type: "text" },
    render: (row) => <span className="block max-w-[220px] truncate" title={row.diffAfter}>{row.diffAfter}</span>,
  },
];

const h3ContractEntries = [
  {
    label: "OpenAPI Spec",
    path: "/openapi.json",
    description: "机器可读契约源，供生成客户端与契约一致性检查使用",
  },
  {
    label: "Swagger UI",
    path: "/api-docs",
    description: "交互式接口调试入口，适合联调与冒烟验证",
  },
  {
    label: "ReDoc",
    path: "/redoc",
    description: "只读文档门户，适合评审接口字段与错误码",
  },
] as const;

export function H2AuditTrailPage() {
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultH2QueryValue, normalizeH2QueryValue);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);

  const queryParams = React.useMemo(() => normalizeH2Query(appliedQuery), [appliedQuery]);
  const eventsQuery = useAuditEventsQuery(queryParams);
  const rows = eventsQuery.data ?? [];
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h2AuditQueryFields, appliedQuery),
    [appliedQuery],
  );

  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    description: "重新加载审计事件",
    disabled: eventsQuery.isFetching,
    onClick: () => {
      void eventsQuery.refetch();
    },
  };

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H2 审计追踪"
        subtitle="append-only 审计作业查询 · GET /api/v1/audit/events"
      />

      <QueryPanel
        fields={h2AuditQueryFields}
        defaultVisibleFieldKeys={h2AuditCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeH2QueryValue(next))}
        onQuery={() => {
          applyQuery(draftQuery);
          setSelectedRowKeys([]);
        }}
        onReset={() => {
          resetQuery();
          setSelectedRowKeys([]);
        }}
        resetLabel="重置"
      />

      <DataGrid
        storageKey="h2.audit-trail.events"
        columns={h2AuditColumns}
        data={rows}
        rowKey={(row) => row.id}
        selectable
        selectedRowKeys={selectedRowKeys}
        onSelectedRowKeysChange={setSelectedRowKeys}
        queryState={appliedQuery}
        querySummaryItems={querySummaryItems}
        refreshAction={refreshAction}
        caption={
          eventsQuery.isPending
            ? "加载审计事件..."
            : eventsQuery.isFetching
              ? "刷新中..."
              : `当前页 ${rows.length} 条 · 按时间倒序`
        }
        emptyTitle={eventsQuery.isError ? "读取审计事件失败" : "暂无匹配的审计事件"}
        emptyDescription={
          eventsQuery.isError
            ? eventsQuery.error.message
            : "可调整对象类型或时间范围后重新查询"
        }
        exportFileBaseName="H2 审计追踪"
        tableClassName="min-w-[1100px]"
      />

      <p className="text-xs text-muted-foreground">
        数据来源接口：GET /api/v1/audit/events；审计记录仅追加，不可修改或删除（GSP 合规）。
      </p>
    </section>
  );
}

export function H3ApiContractPage() {
  const packageVersion = "0.0.1";
  const contractSyncedAt = "与 shared/openapi 同步";

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H3 OpenAPI 契约"
        subtitle="契约门户 · 文档入口 · 前端类型同步说明"
      />

      <div className="grid gap-4 lg:grid-cols-[20rem_1fr]">
        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-4 p-5">
            <div className="flex items-center justify-between gap-3">
              <div className="flex size-10 items-center justify-center rounded-md bg-primary/10 text-primary">
                <FileJson className="size-5" aria-hidden />
              </div>
              <StatusBadge status="completed" label="已同步" size="sm" />
            </div>
            <div>
              <h2 className="text-base font-semibold tracking-normal">契约状态</h2>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">
                后端 OpenAPI 为唯一契约源；前端通过 `@wms/api-client` 消费生成类型，避免手写请求模型漂移。
              </p>
            </div>
            <dl className="space-y-2 text-sm">
              <div className="flex items-center justify-between gap-3">
                <dt className="text-muted-foreground">Web Admin 版本</dt>
                <dd className="font-mono text-xs">{packageVersion}</dd>
              </div>
              <div className="flex items-center justify-between gap-3">
                <dt className="text-muted-foreground">契约同步</dt>
                <dd className="text-xs">{contractSyncedAt}</dd>
              </div>
              <div className="flex items-center justify-between gap-3">
                <dt className="text-muted-foreground">类型包</dt>
                <dd className="font-mono text-xs">@wms/api-client</dd>
              </div>
            </dl>
          </CardContent>
        </Card>

        <Card className="rounded-lg shadow-sm">
          <CardContent className="space-y-3 p-5">
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-base font-semibold tracking-normal">文档与契约入口</h2>
              <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                <RefreshCw className="size-3.5" aria-hidden />
                修改 OpenAPI 后需重新生成客户端类型
              </span>
            </div>
            <div className="grid gap-3 md:grid-cols-3">
              {h3ContractEntries.map((entry) => (
                <a
                  key={entry.path}
                  href={entry.path}
                  target="_blank"
                  rel="noreferrer"
                  className="rounded-md border bg-background px-3 py-3 transition-colors hover:border-primary hover:bg-primary/5"
                >
                  <div className="flex items-center justify-between gap-2 text-sm font-medium">
                    <span className="inline-flex items-center gap-2">
                      <FileJson className="size-4 text-primary" aria-hidden />
                      {entry.label}
                    </span>
                    <ExternalLink className="size-3.5 text-muted-foreground" aria-hidden />
                  </div>
                  <div className="mt-2 truncate font-mono text-xs text-muted-foreground">
                    GET {entry.path}
                  </div>
                  <p className="mt-2 text-xs leading-5 text-muted-foreground">{entry.description}</p>
                </a>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}

function defaultH2QueryValue(): QueryPanelValue {
  return {
    resourceType: "",
    actorId: "",
    action: "",
    resourceId: "",
    productCode: "",
    batchNo: "",
    occurredAt: { from: "", to: "" },
  };
}

function normalizeH2QueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    resourceType: queryString(value.resourceType),
    actorId: queryString(value.actorId),
    action: queryString(value.action),
    resourceId: queryString(value.resourceId),
    productCode: queryString(value.productCode),
    batchNo: queryString(value.batchNo),
    occurredAt: queryRange(value.occurredAt),
  };
}

function normalizeH2Query(value: QueryPanelValue) {
  const normalized = normalizeH2QueryValue(value);
  const range = queryRange(normalized.occurredAt);
  return {
    resourceType: queryString(normalized.resourceType),
    actorId: queryString(normalized.actorId),
    action: queryString(normalized.action),
    resourceId: queryString(normalized.resourceId),
    productCode: queryString(normalized.productCode),
    batchNo: queryString(normalized.batchNo),
    from: toRfc3339Start(range.from ?? ""),
    to: toRfc3339End(range.to ?? ""),
  };
}

// 本地实现保留 trim 语义（公共 queryString 不做 trim）。
function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value.trim() : "";
}

function toRfc3339Start(value: string) {
  if (!value) return undefined;
  if (value.includes("T")) return value;
  return new Date(`${value}T00:00:00`).toISOString();
}

function toRfc3339End(value: string) {
  if (!value) return undefined;
  if (value.includes("T")) return value;
  return new Date(`${value}T23:59:59.999`).toISOString();
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const yyyy = date.getFullYear();
  const mm = String(date.getMonth() + 1).padStart(2, "0");
  const dd = String(date.getDate()).padStart(2, "0");
  const hh = String(date.getHours()).padStart(2, "0");
  const mi = String(date.getMinutes()).padStart(2, "0");
  const ss = String(date.getSeconds()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd} ${hh}:${mi}:${ss}`;
}
