/**
 * M3LocationHistoryPage — 库位历史追踪
 *
 * 层级：Layer 3 页面
 * 关联故事：US-M3-011, US-M3-001
 * Wave：Wave 6
 */

import * as React from "react";
import {
  Card,
  CardContent,
  DataGrid,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";

import {
  useLocationHistoryQuery,
  type LocationHistoryMovement,
  type LocationHistoryQuery,
} from "@/features/inventory/inventory-queries";
import { errorText } from "@/lib/error-text";
import { formatDateTime } from "@/lib/format";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { usePageQueryState } from "@/lib/use-page-query-state";

const PENDING_LOCATION_KEY = "m3-location-history-code";

export const m3LocationHistoryCoreQueryFieldKeys = ["locationCode", "days"];

export const m3LocationHistoryQueryFields: QueryPanelField[] = [
  { key: "locationCode", label: "库位编码", type: "text", placeholder: "输入或扫库位码" },
  {
    key: "days",
    label: "回溯天数",
    type: "select",
    options: [
      { label: "近 7 天", value: "7" },
      { label: "近 30 天", value: "30" },
      { label: "近 90 天", value: "90" },
      { label: "全部(3650)", value: "3650" },
    ],
  },
  { key: "movementType", label: "操作类型", type: "text", placeholder: "inbound_putaway / outbound_ship" },
  { key: "productCode", label: "商品编码", type: "text", placeholder: "按商品编码模糊查询" },
  { key: "batchNo", label: "批号", type: "text", placeholder: "按批号模糊查询" },
];

interface M3LocationHistoryPageProps {
  onBack?: () => void;
  initialLocationCode?: string;
}

export function M3LocationHistoryPage({ onBack, initialLocationCode }: M3LocationHistoryPageProps) {
  void onBack;
  const pendingLocation = React.useMemo(() => {
    if (initialLocationCode?.trim()) return initialLocationCode.trim();
    if (typeof sessionStorage === "undefined") return "";
    const value = sessionStorage.getItem(PENDING_LOCATION_KEY) ?? "";
    if (value) sessionStorage.removeItem(PENDING_LOCATION_KEY);
    return value;
  }, [initialLocationCode]);

  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => defaultQuery(""), normalizeQuery);
  const pendingLocationAppliedRef = React.useRef(false);
  React.useEffect(() => {
    if (!pendingLocation || pendingLocationAppliedRef.current) return;
    pendingLocationAppliedRef.current = true;
    applyQuery(defaultQuery(pendingLocation));
  }, [pendingLocation, applyQuery]);
  const apiQuery = React.useMemo(() => toApiQuery(appliedQuery), [appliedQuery]);
  const enabled = Boolean(apiQuery.location_code);
  const historyQuery = useLocationHistoryQuery(apiQuery, enabled);
  const rows = historyQuery.data?.data ?? [];
  const risks = historyQuery.data?.risks ?? [];
  const productShares = historyQuery.data?.product_shares ?? [];
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m3LocationHistoryQueryFields, appliedQuery),
    [appliedQuery],
  );

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新库位历史",
    disabled: historyQuery.isFetching || !enabled,
    onClick: () => {
      void historyQuery.refetch();
    },
  };

  const columns = React.useMemo<DataGridColumn<LocationHistoryMovement>[]>(
    () => [
      {
        key: "created_at",
        header: "创建时间",
        width: 180,
        minWidth: 160,
        sortable: true,
        sortValue: (row) => row.occurred_at,
        copyValue: (row) => row.occurred_at,
        render: (row) => <span className="font-mono text-xs">{formatDateTime(row.occurred_at)}</span>,
      },
      {
        key: "occurred_at",
        header: "操作时间",
        width: 180,
        minWidth: 160,
        sortable: true,
        sortValue: (row) => row.occurred_at,
        copyValue: (row) => row.occurred_at,
        render: (row) => <span className="font-mono text-xs">{formatDateTime(row.occurred_at)}</span>,
      },
      {
        key: "movement_type",
        header: "操作类型",
        width: 140,
        minWidth: 120,
        sortable: true,
        sortValue: (row) => row.movement_type,
        filterValue: (row) => row.movement_type,
        copyValue: (row) => row.movement_type,
        filter: { type: "text" },
      },
      {
        key: "product_code",
        header: "商品",
        width: 180,
        minWidth: 150,
        render: (row) => (
          <div className="grid gap-0.5">
            <span className="font-mono text-xs">{row.product_code ?? "—"}</span>
            <span className="text-xs text-muted-foreground">{row.product_name ?? "—"}</span>
          </div>
        ),
      },
      {
        key: "batch_no",
        header: "批号/效期",
        width: 160,
        minWidth: 140,
        render: (row) => (
          <div className="grid gap-0.5">
            <span className="font-mono text-xs">{row.batch_no ?? "—"}</span>
            <span className="text-xs text-muted-foreground">{row.expiry_date ?? "—"}</span>
          </div>
        ),
      },
      {
        key: "qty_delta",
        header: "数量变动",
        width: 100,
        minWidth: 90,
        sortable: true,
        sortValue: (row) => row.qty_delta,
        copyValue: (row) => String(row.qty_delta),
        render: (row) => <span className="font-mono">{row.qty_delta > 0 ? `+${row.qty_delta}` : row.qty_delta}</span>,
      },
      {
        key: "location",
        header: "库位",
        width: 180,
        minWidth: 160,
        render: (row) => (
          <div className="grid gap-0.5 font-mono text-xs">
            <span>{row.location_code ?? "—"}</span>
            <span className="text-muted-foreground">
              {row.from_location_code ?? "—"} → {row.to_location_code ?? "—"}
            </span>
          </div>
        ),
      },
      {
        key: "lpn_code",
        header: "LPN",
        width: 120,
        minWidth: 100,
        copyValue: (row) => row.lpn_code ?? "",
        render: (row) => <span className="font-mono text-xs">{row.lpn_code ?? "—"}</span>,
      },
      {
        key: "operator_name",
        header: "操作人",
        width: 120,
        minWidth: 100,
        render: (row) => row.operator_name ?? "—",
      },
      {
        key: "source_document_type",
        header: "关联单据",
        width: 160,
        minWidth: 140,
        render: (row) => (
          <div className="grid gap-0.5">
            <span>{row.source_document_type}</span>
            <span className="font-mono text-xs text-muted-foreground">{row.source_document_id}</span>
          </div>
        ),
      },
      {
        key: "volume_delta_cm3",
        header: "容积变化",
        width: 110,
        minWidth: 100,
        render: (row) => row.volume_delta_cm3 ?? "—",
      },
    ],
    [],
  );

  return (
    <div className="space-y-4">
      <PageHeader
        title="M3 库位历史追踪"
        subtitle="按库位反查商品/批号变动、风险与导出证据"
      />
      <QueryPanel
        fields={m3LocationHistoryQueryFields}
        defaultVisibleFieldKeys={m3LocationHistoryCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => applyQuery(draftQuery)}
        onReset={resetQuery}
      />
      {risks.length > 0 && (
        <section className="rounded-lg border border-amber-300 bg-amber-50 p-4" aria-label="库位风险识别">
          <h2 className="font-semibold text-amber-900">风险识别</h2>
          <ul className="mt-2 list-disc space-y-1 pl-5 text-sm text-amber-950">
            {risks.map((risk) => (
              <li key={risk.risk_code}>
                <strong>{risk.severity}</strong> · {risk.message}
              </li>
            ))}
          </ul>
        </section>
      )}
      {productShares.length > 0 && (
        <section className="rounded-lg border bg-card p-4" aria-label="商品分布">
          <h2 className="font-semibold">商品分布</h2>
          <ul className="mt-2 grid gap-1 text-sm sm:grid-cols-2 lg:grid-cols-3">
            {productShares.map((share) => (
              <li key={share.product_code} className="rounded border px-3 py-2">
                <div className="font-mono text-xs">{share.product_code}</div>
                <div className="text-muted-foreground">{share.product_name ?? "—"}</div>
                <div>事件 {share.event_count} · 数量合计 {share.total_qty_delta}</div>
              </li>
            ))}
          </ul>
        </section>
      )}
      <Card>
        <CardContent className="p-5">
          <DataGrid
            storageKey="m3.location-history"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            caption={historyQuery.isPending && enabled ? "加载库位历史..." : undefined}
            emptyTitle={!enabled ? "请输入库位编码" : historyQuery.isError ? "读取库位历史失败" : "暂无库位历史"}
            emptyDescription={
              !enabled
                ? "从 M3 批号管理库位专项视图跳转，或在此输入库位编码后查询"
                : historyQuery.isError
                  ? errorText(historyQuery.error, "请检查鉴权和 API 服务")
                  : "该库位在所选时间范围内没有流水"
            }
            exportFileBaseName="M3-库位历史"
            refreshAction={refreshAction}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={(value) => applyQuery(queryValueFromUnknown(value))}
            onClearQueryState={resetQuery}
          />
        </CardContent>
      </Card>
    </div>
  );
}

export function rememberLocationHistoryCode(locationCode: string) {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(PENDING_LOCATION_KEY, locationCode.trim());
}

function defaultQuery(locationCode: string): QueryPanelValue {
  return {
    locationCode,
    days: "30",
    movementType: "",
    productCode: "",
    batchNo: "",
  };
}

function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  return {
    locationCode: queryString(value.locationCode).trim(),
    days: queryString(value.days).trim() || "30",
    movementType: queryString(value.movementType).trim(),
    productCode: queryString(value.productCode).trim(),
    batchNo: queryString(value.batchNo).trim(),
  };
}

function toApiQuery(query: QueryPanelValue): LocationHistoryQuery {
  const daysRaw = Number(queryString(query.days) || "30");
  return {
    location_code: queryString(query.locationCode).trim() || undefined,
    days: Number.isFinite(daysRaw) ? daysRaw : 30,
    movement_type: queryString(query.movementType).trim() || undefined,
    product_code: queryString(query.productCode).trim() || undefined,
    batch_no: queryString(query.batchNo).trim() || undefined,
  };
}
