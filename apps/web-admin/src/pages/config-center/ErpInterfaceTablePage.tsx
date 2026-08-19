/** US-H8-004：受控接口表只读探查；只展示摘要，不提供 SQL 或写操作。 */
import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  formatDateTime,
  ListPageTemplate,
  type DataGridColumn,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";
import { Database, Eye, RefreshCw } from "lucide-react";

import {
  useH8ErpInterfaceTableConnectorsQuery,
  useH8ErpInterfaceTableDetailQuery,
  useH8ErpInterfaceTableRowsQuery,
  type H8ErpInterfaceTableDetail,
  type H8ErpInterfaceTableRow,
} from "@/features/config-center/erp-interface-table-queries";
import { usePageQueryState } from "@/lib/use-page-query-state";
import {
  BUTTON_REFRESH, COLUMN_CREATED_AT,
  COLUMN_UPDATED_AT, FIELD_WAREHOUSE_ID, STATUS_DEACTIVATED, STATUS_PENDING,
} from "@/lib/ui-strings";

const TABLES = [
  ["x_wmsinter_GoodsInfo", "商品主数据"],
  ["x_wmsinter_CustomerInfo", "客户主数据"],
  ["x_wmsinter_SupplierInfo", "供应商主数据"],
  ["x_wmsinter_InboundOrder", "入库单头"],
  ["x_wmsinter_InboundOrderItems", "入库单明细"],
  ["x_wmsinter_OutboundOrder", "出库单头"],
  ["x_wmsinter_OutboundOrderItems", "出库单明细"],
  ["x_wmsinter_OrderFeedback", "订单状态反馈"],
  ["x_wmsinter_OrderCommand", "订单命令"],
  ["x_wmsinter_InboundFeedback", "入库明细反馈"],
  ["x_wmsinter_OutboundFeedback", "出库明细反馈"],
  ["x_wmsinter_WmsEvent", "WMS 事件"],
  ["x_wmsinter_InventoryPushHeader", "ERP 库存快照头"],
  ["x_wmsinter_InventoryPushItems", "ERP 库存快照明细"],
  ["x_wmsinter_InventoryReceiveHeader", "WMS 库存快照头"],
  ["x_wmsinter_InventoryReceiveItems", "WMS 库存快照明细"],
] as const;

const MAIN_TABLES = new Set([
  "x_wmsinter_GoodsInfo",
  "x_wmsinter_CustomerInfo",
  "x_wmsinter_SupplierInfo",
  "x_wmsinter_InboundOrder",
  "x_wmsinter_OutboundOrder",
  "x_wmsinter_OrderFeedback",
  "x_wmsinter_OrderCommand",
  "x_wmsinter_InboundFeedback",
  "x_wmsinter_OutboundFeedback",
  "x_wmsinter_WmsEvent",
  "x_wmsinter_InventoryPushHeader",
  "x_wmsinter_InventoryReceiveHeader",
]);
const V19_STATUSES = ["pending", "processing", "awaiting_receipt", "failed", "dead", "acked"];
const STATUS_LABELS: Record<string, string> = {
  pending: STATUS_PENDING,
  processing: "处理中",
  awaiting_receipt: "技术接收完成",
  failed: "可重试失败",
  dead: "死信",
  acked: "业务已提交",
  readonly: "只读子记录",
  unknown: "未知状态",
  testing: "测试中",
  active: "已启用",
  disabled: STATUS_DEACTIVATED,
};
const DETAIL_FIELD_LABELS: Record<string, string> = {
  id: "记录 ID",
  owner_id: "WMS 货主 ID",
  owner_code: "ERP 货主编码",
  business_key: "业务键摘要",
  event_type: "事件类型",
  external_ref: "外部引用",
  warehouse_id: FIELD_WAREHOUSE_ID,
  wms_resource_id: "WMS 资源 ID",
  sync_status: "同步状态",
  retry_count: "重试次数",
  last_error: "错误摘要",
  idempotency_key: "幂等键",
  created_at: COLUMN_CREATED_AT,
  updated_at: COLUMN_UPDATED_AT,
  payload_summary: "报文摘要",
  product_code: "商品编码",
  product_name: "商品名称",
  spec: "规格",
  approval_no: "批准文号",
  manufacturer: "生产厂家",
  special_drug_category: "特殊药品分类",
  storage_condition: "储存条件",
  schema_version: "契约版本",
};

function statusLabel(status: string | null | undefined): string {
  return status ? STATUS_LABELS[status] ?? status : "—";
}

function businessValue(row: H8ErpInterfaceTableRow, key: string): string {
  return row.business_fields.find((field) => field.key === key)?.value ?? "—";
}

export const h8ErpInterfaceTableQueryFields: QueryPanelField[] = [
  { key: "connector_id", label: "连接", type: "multiSelect", options: [{ label: "请选择", value: "" }] },
  {
    key: "table_key",
    label: "接口表",
    type: "multiSelect",
    options: TABLES.map(([value, label]) => ({ value, label: `${label}（${value}）` })),
  },
  { key: "sync_status", label: "同步状态", type: "multiSelect", options: [] },
  { key: "updated_at", label: "写入时间（最近 7 天）", type: "dateRange" },
  { key: "external_doc_no", label: "外部单据号", type: "text" },
  { key: "event_type", label: "事件类型", type: "text" },
  { key: "idempotency_key", label: "幂等键", type: "text" },
];
const h8ErpInterfaceTableQueryFieldDefinitions = h8ErpInterfaceTableQueryFields;

export const h8ErpInterfaceTableCoreQueryFieldKeys = [
  "connector_id",
  "table_key",
  "sync_status",
  "updated_at",
];

const columns: DataGridColumn<H8ErpInterfaceTableRow>[] = [
  { key: "row_id", header: "记录 ID", width: 230, mono: true },
  { key: "business_key", header: "业务键摘要", width: 180, render: (row) => row.business_key ?? "—" },
  { key: "event_type", header: "事件类型", width: 150, render: (row) => row.event_type ?? "—" },
  { key: "sync_status", header: "同步状态", width: 110, render: (row) => statusLabel(row.sync_status) },
  { key: "retry_count", header: "重试次数", width: 90 },
  { key: "last_error", header: "错误摘要", width: 220, render: (row) => row.last_error ?? "—" },
  { key: "idempotency_key", header: "幂等键", width: 180, render: (row) => row.idempotency_key ?? "—" },
  { key: "created_at", header: COLUMN_CREATED_AT, width: 175, render: (row) => formatDateTime(row.created_at) },
  { key: "updated_at", header: COLUMN_UPDATED_AT, width: 175, render: (row) => formatDateTime(row.updated_at) },
];

const productColumns: DataGridColumn<H8ErpInterfaceTableRow>[] = [
  { key: "row_id", header: "记录 ID", width: 110, mono: true },
  { key: "owner_code", header: "ERP 货主编码", width: 130, render: (row) => businessValue(row, "owner_code") },
  { key: "product_code", header: "商品编码", width: 160, render: (row) => businessValue(row, "product_code") },
  { key: "product_name", header: "商品名称", width: 240, render: (row) => businessValue(row, "product_name") },
  { key: "spec", header: "规格", width: 160, render: (row) => businessValue(row, "spec") },
  { key: "sync_status", header: "同步状态", width: 110, render: (row) => statusLabel(row.sync_status) },
  { key: "retry_count", header: "重试次数", width: 90 },
  { key: "updated_at", header: COLUMN_UPDATED_AT, width: 175, render: (row) => formatDateTime(row.updated_at) },
];

type PackagingLevel = {
  unit: string;
  ratio_to_base: number;
  is_base: boolean;
  is_default: boolean;
  sort_order: number;
};

const PRODUCT_DETAIL_GROUPS = [
  ["商品信息", ["business_key", "product_code", "product_name", "spec"]],
  [
    "药品与监管",
    [
      "approval_no",
      "manufacturer",
      "special_drug_category",
      "storage_condition",
    ],
  ],
  ["物流与包装", []],
  [
    "同步追踪",
    [
      "schema_version",
      "owner_code",
      "owner_id",
      "sync_status",
      "retry_count",
      "last_error",
      "idempotency_key",
      "created_at",
      "updated_at",
      "payload_summary",
    ],
  ],
] as const;

function packagingLevels(value: string | null | undefined): PackagingLevel[] | null {
  if (!value) return null;
  try {
    const parsed: unknown = JSON.parse(value);
    if (
      !Array.isArray(parsed) ||
      !parsed.every(
        (level): level is PackagingLevel =>
          typeof level === "object" &&
          level !== null &&
          typeof level.unit === "string" &&
          typeof level.ratio_to_base === "number" &&
          typeof level.is_base === "boolean" &&
          typeof level.is_default === "boolean" &&
          typeof level.sort_order === "number",
      )
    ) return null;
    return parsed;
  } catch {
    return null;
  }
}

function detailValue(key: string, value: string | null | undefined): string {
  if (key === "sync_status") return statusLabel(value);
  if ((key === "created_at" || key === "updated_at") && value) return formatDateTime(value);
  return value ?? "—";
}

function ProductMasterDetail({ detail }: { detail: H8ErpInterfaceTableDetail }) {
  const values = new Map(detail.fields.map((field) => [field.key, field.value]));
  const packaging = packagingLevels(values.get("packaging_levels"));
  return (
    <div className="grid max-h-[65vh] gap-3 overflow-auto text-sm">
      {PRODUCT_DETAIL_GROUPS.map(([title, keys]) => (
        <div key={title} className="rounded-md border p-3">
          <h3 className="mb-2 font-semibold">{title}</h3>
          <div className="grid gap-2 sm:grid-cols-2">
            {keys.map((key) => (
              <div key={key} className="grid grid-cols-[8rem_1fr] gap-2">
                <span className="text-muted-foreground">{DETAIL_FIELD_LABELS[key]}</span>
                <span className="break-all">{detailValue(key, values.get(key))}</span>
              </div>
            ))}
          </div>
          {title === "物流与包装" ? (
            <div className="mt-3">
              <h4 className="mb-2 font-medium">包装层级</h4>
              {packaging?.length ? (
                <div className="overflow-x-auto rounded border">
                  <table aria-label="包装层级" className="w-full text-left">
                    <thead className="bg-muted/50">
                      <tr>
                        {["包装单位", "基本单位换算", "基本单位", "默认单位"].map((label) => (
                          <th key={label} className="px-2 py-1.5 font-medium">{label}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {packaging.map((level) => (
                        <tr key={`${level.sort_order}-${level.unit}`} className="border-t">
                          <td className="px-2 py-1.5">{level.unit}</td>
                          <td className="px-2 py-1.5">{level.ratio_to_base}</td>
                          <td className="px-2 py-1.5">{level.is_base ? "是" : "否"}</td>
                          <td className="px-2 py-1.5">{level.is_default ? "是" : "否"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p className="text-muted-foreground">{values.get("packaging_levels") ?? "—"}</p>
              )}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}

type Connector = {
  id: string;
  connector_code: string;
  connector_name: string;
  channel_mode: string;
  status: string;
  probe_credentials_configured: boolean;
};

function asRange(value: QueryPanelValue["updated_at"]): QueryPanelRangeValue {
  return value && typeof value === "object" && !Array.isArray(value) ? value : {};
}

/** 与 ErpMessageLogPage.toIsoDay 一致：按本地时区解析日界，避免 UTC 边界漏掉当天数据。 */
function toIsoDay(value: string | undefined, end = false): string | undefined {
  if (!value) return undefined;
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(
    year,
    month - 1,
    day,
    end ? 23 : 0,
    end ? 59 : 0,
    end ? 59 : 0,
    end ? 999 : 0,
  );
  return Number.isNaN(date.getTime()) ? undefined : date.toISOString();
}

function defaultQuery(connectorId = ""): QueryPanelValue {
  return {
    connector_id: connectorId,
    table_key: "x_wmsinter_GoodsInfo",
    sync_status: [],
    updated_at: {},
    external_doc_no: "",
    event_type: "",
    idempotency_key: "",
  };
}

function isApplicable(tableKey: string, key: string): boolean {
  if (key === "idempotency_key") return true;
  if (key === "event_type") return tableKey === "x_wmsinter_WmsEvent";
  if (key !== "external_doc_no") return false;
  return [
    "x_wmsinter_InboundOrder",
    "x_wmsinter_InboundOrderItems",
    "x_wmsinter_OutboundOrder",
    "x_wmsinter_OutboundOrderItems",
    "x_wmsinter_OrderFeedback",
    "x_wmsinter_OrderCommand",
    "x_wmsinter_InboundFeedback",
    "x_wmsinter_OutboundFeedback",
  ].includes(tableKey);
}

function queryErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error && error.message.trim() ? `${fallback}：${error.message}` : fallback;
}

export function ErpInterfaceTablePage() {
  const connectorsQuery = useH8ErpInterfaceTableConnectorsQuery();
  const connectors = (connectorsQuery.data ?? []) as Connector[];
  const firstReadyConnectorId = connectors.find((connector) => connector.probe_credentials_configured)?.id ?? "";
  const { draftQuery, setDraftQuery, appliedQuery, setAppliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => defaultQuery(firstReadyConnectorId));
  const [detailId, setDetailId] = React.useState<string | null>(null);
  const [selectedKeys, setSelectedKeys] = React.useState<string[]>([]);

  React.useEffect(() => {
    // 连接列表返回后只补齐 connector_id，不得整体重置查询：用户已填的其他条件必须保留。
    if (!String(draftQuery.connector_id) && firstReadyConnectorId) {
      setDraftQuery((prev) => ({ ...prev, connector_id: firstReadyConnectorId }));
      setAppliedQuery((prev) => ({ ...prev, connector_id: firstReadyConnectorId }));
    }
  }, [draftQuery.connector_id, firstReadyConnectorId, setAppliedQuery, setDraftQuery]);

  const tableKey = String(appliedQuery.table_key ?? "x_wmsinter_GoodsInfo");
  const selectedConnector = connectors.find((connector) => connector.id === String(appliedQuery.connector_id ?? ""));
  const selectedConnectorReady = selectedConnector?.probe_credentials_configured === true;
  const range = asRange(appliedQuery.updated_at);
  const listQuery = useH8ErpInterfaceTableRowsQuery({
    connector_id: String(appliedQuery.connector_id ?? ""),
    table_key: tableKey,
    sync_status: MAIN_TABLES.has(tableKey) && Array.isArray(appliedQuery.sync_status)
      ? appliedQuery.sync_status.join(",") || undefined
      : undefined,
    time_from: toIsoDay(range.from),
    time_to: toIsoDay(range.to, true),
    external_doc_no: isApplicable(tableKey, "external_doc_no") ? String(appliedQuery.external_doc_no ?? "") || undefined : undefined,
    event_type: isApplicable(tableKey, "event_type") ? String(appliedQuery.event_type ?? "") || undefined : undefined,
    idempotency_key: String(appliedQuery.idempotency_key ?? "") || undefined,
    page: 1,
    page_size: 50,
  });
  const detailQuery = useH8ErpInterfaceTableDetailQuery(String(appliedQuery.connector_id ?? ""), tableKey, detailId);
  const rows = listQuery.data?.items ?? [];
  const selected = rows.find((row) => row.row_id === selectedKeys[0]);
  const draftTableKey = String(draftQuery.table_key ?? "x_wmsinter_GoodsInfo");
  // 查询闸门必须依据草稿里选中的连接：按 appliedQuery 判断会在“已应用连接不可用”时死锁，永远无法查询。
  const draftConnector = connectors.find((connector) => connector.id === String(draftQuery.connector_id ?? ""));
  const draftConnectorReady = draftConnector?.probe_credentials_configured === true;
  const h8ErpInterfaceTableQueryFields = h8ErpInterfaceTableQueryFieldDefinitions
    .filter((field) =>
      (h8ErpInterfaceTableCoreQueryFieldKeys.includes(field.key) &&
        (field.key !== "sync_status" || MAIN_TABLES.has(draftTableKey))) ||
      isApplicable(draftTableKey, field.key),
    )
    .map((field) =>
      field.key === "connector_id"
      ? { ...field, options: [{ label: "请选择", value: "" }, ...connectors.map((connector) => ({ label: connector.probe_credentials_configured ? `${connector.connector_name}（${connector.connector_code} · ${statusLabel(connector.status)}）` : `${connector.connector_name}（${connector.connector_code} · 未配置独立探查凭据）`, value: connector.id, disabled: !connector.probe_credentials_configured }))] }
      : field.key === "sync_status"
        ? { ...field, options: V19_STATUSES.map((value) => ({ label: statusLabel(value), value })) }
      : field,
    );
  const toolbarActions: DataGridToolbarAction[] = [
    { key: "refresh", label: BUTTON_REFRESH, icon: <RefreshCw className="size-4" aria-hidden />, onClick: () => void listQuery.refetch() },
    { key: "detail", label: "详情", icon: <Eye className="size-4" aria-hidden />, disabled: (ctx) => ctx.selectedRowKeys.length !== 1, onClick: () => selected && setDetailId(selected.row_id) },
  ];

  return (
    <ListPageTemplate
      key={tableKey}
      header={{
        subtitle: "MSSQL 只读查询 · 最近 7 天 · 最大跨度 31 天 · 无写操作",
        actions: (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Database className="size-4" aria-hidden />
            合计 {listQuery.data?.total ?? 0}
          </div>
        ),
      }}
      banner={
        <>
          {selectedConnector && !selectedConnectorReady ? (
            <div role="status" className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">
              当前连接未配置独立探查凭据；请由系统管理员在 H8 ERP 连接页维护成对的探查账号与密码别名。
            </div>
          ) : null}
          {selectedConnector && selectedConnector.status !== "active" ? (
            <div role="status" className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900">
              连接状态为 {statusLabel(selectedConnector.status)}；探查仍允许用于排障，但请确认接口库只读凭据和网络状态。
            </div>
          ) : null}
          {listQuery.isError ? (
            <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {queryErrorMessage(listQuery.error, "接口表读取失败")}
            </div>
          ) : null}
        </>
      }
      queryFields={h8ErpInterfaceTableQueryFields}
      coreQueryFieldKeys={h8ErpInterfaceTableCoreQueryFieldKeys}
      queryValue={draftQuery}
      onQueryValueChange={(next) =>
        setDraftQuery(
          MAIN_TABLES.has(String(next.table_key ?? "x_wmsinter_GoodsInfo"))
            ? next
            : { ...next, sync_status: [] },
        )
      }
      onQuery={() => {
        if (draftConnectorReady) applyQuery(draftQuery);
      }}
      onReset={resetQuery}
      gridProps={{
        storageKey: `h8.erp-interface-tables.${tableKey}`,
        columns: tableKey === "x_wmsinter_GoodsInfo" ? productColumns : columns,
        data: rows,
        rowKey: (row) => row.row_id,
        selectable: true,
        selectedRowKeys: selectedKeys,
        onSelectedRowKeysChange: setSelectedKeys,
        toolbarActions,
        emptyTitle: listQuery.isLoading ? "加载接口表…" : "暂无接口表行",
        emptyDescription: "仅展示受控接口表的脱敏控制列和摘要",
      }}
      dialogs={
        <Dialog open={detailId != null} onOpenChange={(open) => !open && setDetailId(null)}>
          <DialogContent className="sm:max-w-2xl">
            <DialogHeader>
              <DialogTitle>{tableKey === "x_wmsinter_GoodsInfo" ? "商品主数据接口行详情" : "接口表行详情"}</DialogTitle>
              <DialogDescription>联合身份：连接 + 表 + 行 ID；仅展示服务端白名单业务字段和脱敏摘要。</DialogDescription>
            </DialogHeader>
            {detailQuery.data ? (
              tableKey === "x_wmsinter_GoodsInfo" ? (
                <ProductMasterDetail detail={detailQuery.data} />
              ) : (
                <div className="grid max-h-[60vh] gap-2 overflow-auto text-sm">{detailQuery.data.fields.map((field) => <div key={field.key} className="grid grid-cols-[10rem_1fr] gap-2 rounded border px-2 py-1"><span className="font-medium">{DETAIL_FIELD_LABELS[field.key] ?? "其他字段"}</span><span className="break-all">{detailValue(field.key, field.value)}</span></div>)}</div>
              )
            ) : detailQuery.isError ? <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">{queryErrorMessage(detailQuery.error, "详情读取失败")}</div> : <p className="text-sm text-muted-foreground">加载中…</p>}
            <DialogFooter><DialogClose asChild><Button type="button" variant="outline">关闭</Button></DialogClose></DialogFooter>
          </DialogContent>
        </Dialog>
      }
    />
  );
}
