import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  formatZhDate,
  type DataGridColumn,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Ban, PackageCheck, Printer, Route, Truck } from "lucide-react";

import {
  useCancelExpressWaybillMutation,
  useCreateExpressWaybillMutation,
  useExpressCarriersQuery,
  useExpressRoutingRulesQuery,
  useExpressTrackingMutation,
  useUpsertExpressCarrierMutation,
  useUpsertExpressRoutingRuleMutation,
  type CreateExpressWaybillRequest,
  type ExpressCarrier,
  type ExpressRoutingRule,
  type ExpressTrackingResponse,
  type ExpressWaybill,
  type UpsertExpressCarrierRequest,
  type UpsertExpressRoutingRuleRequest,
} from "@/features/express/express-queries";
import {
  CarrierDialog,
  CancelWaybillDialog,
  RuleDialog,
  TrackingDialog,
  WaybillPrintDialog,
  WaybillDialog,
  type CarrierForm,
  type RuleForm,
  type WaybillForm,
} from "./H5ExpressDialogs";
import { providerLabel, providerOptions, waybillStatusKey, waybillStatusLabel } from "./h5-express-model";
import { errorText } from "@/lib/error-text";
import { queryString as libQueryString, queryStringArray, queryValueFromUnknown } from "@/lib/query-value";
import {
  BUTTON_ADD,
  BUTTON_REFRESH,
  COLUMN_CREATED_AT,
  COLUMN_RULE_NAME,
  COLUMN_STATUS,
  COLUMN_UPDATED_AT,
  FIELD_KEYWORD,
  STATUS_DISABLED,
  STATUS_ENABLED,
} from "@/lib/ui-strings";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";

type Notice = { type: "success" | "error"; text: string } | null;

const queryFields: QueryPanelField[] = [
  { key: "q", label: FIELD_KEYWORD, type: "text", placeholder: "快递商 / 规则编码" },
  {
    key: "enabled",
    label: "启停状态",
    type: "multiSelect",
    options: [{ label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }],
  },
];
const h5ExpressCoreQueryFieldKeys = ["q", "enabled"];

const carrierColumns: DataGridColumn<ExpressCarrier>[] = [
  {
    key: "carrier_code",
    header: "快递商编码",
    width: 160,
    minWidth: 120,
    mono: true,
    sortable: true,
    sortValue: (row) => row.carrier_code,
    filterValue: (row) => row.carrier_code,
    copyValue: (row) => row.carrier_code,
    filter: { type: "text" },
  },
  {
    key: "carrier_name",
    header: "快递商名称",
    width: 200,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.carrier_name,
    filterValue: (row) => row.carrier_name,
    copyValue: (row) => row.carrier_name,
    filter: { type: "text" },
  },
  {
    key: "enabled",
    header: COLUMN_STATUS,
    width: 110,
    minWidth: 90,
    filterValue: (row) => row.enabled ? "true" : "false",
    copyValue: (row) => row.enabled ? STATUS_ENABLED : STATUS_DISABLED,
    filter: { type: "multiSelect", options: [{ label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" />,
  },
  {
    key: "priority",
    header: "优先级",
    width: 110,
    minWidth: 90,
    sortable: true,
    sortValue: (row) => row.priority,
    filterValue: (row) => row.priority,
    filter: { type: "numberRange" },
  },
  {
    key: "api_url",
    header: "接口地址",
    width: 300,
    minWidth: 180,
    filterValue: (row) => row.api_url,
    copyValue: (row) => row.api_url,
    filter: { type: "text" },
    render: (row) => <span className="line-clamp-1">{row.api_url}</span>,
  },
  {
    key: "account_no",
    header: "账号",
    width: 160,
    minWidth: 120,
    filterValue: (row) => row.account_no ?? "",
    copyValue: (row) => row.account_no ?? "",
    filter: { type: "text" },
    render: (row) => row.account_no || "-",
  },
  {
    key: "updated_at",
    header: COLUMN_UPDATED_AT,
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.updated_at,
    copyValue: (row) => formatZhDate(row.updated_at),
    filterValue: (row) => row.updated_at,
    filter: { type: "dateRange" },
    render: (row) => formatZhDate(row.updated_at),
  },
  {
    key: "created_at",
    header: COLUMN_CREATED_AT,
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.created_at,
    copyValue: (row) => formatZhDate(row.created_at),
    filterValue: (row) => row.created_at,
    filter: { type: "dateRange" },
    render: (row) => formatZhDate(row.created_at),
  },
];

const ruleColumns: DataGridColumn<ExpressRoutingRule>[] = [
  {
    key: "rule_code",
    header: "规则编码",
    width: 160,
    minWidth: 120,
    mono: true,
    sortable: true,
    sortValue: (row) => row.rule_code,
    filterValue: (row) => row.rule_code,
    copyValue: (row) => row.rule_code,
    filter: { type: "text" },
  },
  {
    key: "rule_name",
    header: COLUMN_RULE_NAME,
    width: 220,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.rule_name,
    filterValue: (row) => row.rule_name,
    copyValue: (row) => row.rule_name,
    filter: { type: "text" },
  },
  {
    key: "delivery_provider_type",
    header: "配送方式",
    width: 140,
    minWidth: 110,
    filterValue: (row) => row.delivery_provider_type,
    copyValue: (row) => providerLabel(row.delivery_provider_type),
    filter: { type: "multiSelect", options: providerOptions },
    render: (row) => providerLabel(row.delivery_provider_type),
  },
  {
    key: "carrier_code",
    header: "快递商",
    width: 130,
    minWidth: 100,
    mono: true,
    filterValue: (row) => row.carrier_code ?? "",
    copyValue: (row) => row.carrier_code ?? "",
    filter: { type: "text" },
    render: (row) => row.carrier_code || "-",
  },
  {
    key: "enabled",
    header: COLUMN_STATUS,
    width: 110,
    minWidth: 90,
    filterValue: (row) => row.enabled ? "true" : "false",
    copyValue: (row) => row.enabled ? STATUS_ENABLED : STATUS_DISABLED,
    filter: { type: "multiSelect", options: [{ label: STATUS_ENABLED, value: "true" }, { label: STATUS_DISABLED, value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? STATUS_ENABLED : STATUS_DISABLED} size="sm" />,
  },
  {
    key: "priority",
    header: "优先级",
    width: 110,
    minWidth: 90,
    sortable: true,
    sortValue: (row) => row.priority,
    filterValue: (row) => row.priority,
    filter: { type: "numberRange" },
  },
  {
    key: "fallback_strategy",
    header: "兜底策略",
    width: 180,
    minWidth: 130,
    filterValue: (row) => row.fallback_strategy ?? "",
    copyValue: (row) => row.fallback_strategy ?? "",
    filter: { type: "text" },
    render: (row) => row.fallback_strategy || "-",
  },
  {
    key: "created_at",
    header: COLUMN_CREATED_AT,
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.created_at,
    copyValue: (row) => formatZhDate(row.created_at),
    filterValue: (row) => row.created_at,
    filter: { type: "dateRange" },
    render: (row) => formatZhDate(row.created_at),
  },
];

export function H5ExpressPage() {
  const { draftQuery: query, setDraftQuery: setQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultQuery, normalizeQuery);
  const [selectedCarrierKeys, setSelectedCarrierKeys] = React.useState<string[]>([]);
  const [selectedRuleKeys, setSelectedRuleKeys] = React.useState<string[]>([]);
  const carrierDialog = useDialogState<CarrierForm>();
  const ruleDialog = useDialogState<RuleForm>();
  const waybillDialog = useDialogState<WaybillForm>();
  const [trackingDialogOpen, setTrackingDialogOpen] = React.useState(false);
  const [cancelDialogOpen, setCancelDialogOpen] = React.useState(false);
  const [printDialogOpen, setPrintDialogOpen] = React.useState(false);
  const [notice, setNotice] = React.useState<Notice>(null);
  // 弹窗提交失败的提示渲染在弹窗内部（页面层 notice 只保留成功/刷新提示）。
  const [dialogError, setDialogError] = React.useState<string | null>(null);
  const [recentWaybill, setRecentWaybill] = React.useState<ExpressWaybill | null>(null);
  const [tracking, setTracking] = React.useState<ExpressTrackingResponse | null>(null);

  const params = queryParams(appliedQuery);
  const carriersQuery = useExpressCarriersQuery({ q: params.q, enabled: params.enabled });
  const rulesQuery = useExpressRoutingRulesQuery({ q: params.q, enabled: params.enabled });
  const upsertCarrier = useUpsertExpressCarrierMutation();
  const upsertRule = useUpsertExpressRoutingRuleMutation();
  const createWaybill = useCreateExpressWaybillMutation();
  const cancelWaybill = useCancelExpressWaybillMutation();
  const trackingMutation = useExpressTrackingMutation();
  const carrierById = React.useMemo(() => new Map((carriersQuery.data ?? []).map((row) => [row.id, row])), [carriersQuery.data]);
  const ruleById = React.useMemo(() => new Map((rulesQuery.data ?? []).map((row) => [row.id, row])), [rulesQuery.data]);
  const selectedCarrier = selectedCarrierKeys.length === 1 ? carrierById.get(selectedCarrierKeys[0]) ?? null : null;
  const selectedRule = selectedRuleKeys.length === 1 ? ruleById.get(selectedRuleKeys[0]) ?? null : null;

  const canCreateWaybill = Boolean(selectedCarrier?.enabled) && !createWaybill.isPending;
  const canTrackWaybill = Boolean(recentWaybill) && !trackingMutation.isPending;
  const canCancelWaybill = Boolean(recentWaybill) && recentWaybill?.status !== "cancelled" && !cancelWaybill.isPending;
  const canPrintWaybill = Boolean(recentWaybill);

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader />
      <NoticePanel notice={notice} />
      <QueryPanel
        fields={queryFields}
        defaultVisibleFieldKeys={h5ExpressCoreQueryFieldKeys}
        value={query}
        onValueChange={(next) => setQuery(normalizeQuery(next))}
        onQuery={() => applyQuery(query)}
        onReset={resetQuery}
      />

      <Card className="rounded-lg shadow-sm">
        <CardContent className="space-y-4 p-5">
          <SectionTitle icon={<PackageCheck className="size-4" aria-hidden />} title="运单作业" />
          <div className="flex flex-col gap-3 rounded-md border border-border/70 bg-muted/20 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
            <div className="space-y-1 text-sm">
              <div className="text-muted-foreground">最近运单</div>
              {recentWaybill ? (
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-foreground">{recentWaybill.waybill_no}</span>
                  <StatusBadge
                    status={waybillStatusKey(recentWaybill.status)}
                    label={waybillStatusLabel(recentWaybill.status)}
                    size="sm"
                  />
                  <span className="text-muted-foreground">快递商 {recentWaybill.carrier_code}</span>
                </div>
              ) : (
                <div className="text-muted-foreground">尚未生成运单；请先在下方选中已启用快递商后下单</div>
              )}
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                size="sm"
                disabled={!canCreateWaybill}
                onClick={() => selectedCarrier && openWaybillDialog(selectedCarrier)}
              >
                <PackageCheck className="size-4" aria-hidden />
                下单
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={!canTrackWaybill}
                onClick={openTrackingDialog}
              >
                <Route className="size-4" aria-hidden />
                轨迹
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={!canCancelWaybill}
                onClick={openCancelDialog}
              >
                <Ban className="size-4" aria-hidden />
                取消
              </Button>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={!canPrintWaybill}
                onClick={() => setPrintDialogOpen(true)}
              >
                <Printer className="size-4" aria-hidden />
                打印面单
              </Button>
            </div>
          </div>
          {!selectedCarrier?.enabled && (
            <p className="text-xs text-muted-foreground">下单依赖快递商配置表中选中且已启用的快递商。</p>
          )}
        </CardContent>
      </Card>

      <Card className="rounded-lg shadow-sm">
        <CardContent className="space-y-4 p-5">
          <SectionTitle icon={<Truck className="size-4" aria-hidden />} title="快递商配置" />
          <DataGrid
              storageKey="h5.express.carriers"
              columns={carrierColumns}
              data={carriersQuery.data ?? []}
              rowKey={(row) => row.id}
              selectable
              selectedRowKeys={selectedCarrierKeys}
              onSelectedRowKeysChange={setSelectedCarrierKeys}
              caption={carriersQuery.isPending ? "加载快递商配置..." : undefined}
              emptyTitle="暂无快递商配置"
              exportFileBaseName="H5 快递商配置"
              tableClassName="min-w-[1220px]"
              refreshAction={{
                label: BUTTON_REFRESH,
                description: "刷新快递商配置",
                disabled: carriersQuery.isFetching,
                onClick: () => void refreshCarriers(),
              }}
              createAction={{ label: BUTTON_ADD, description: "新增快递商", onClick: () => openCarrierDialog(null) }}
              editAction={{
                label: "修改",
                description: "修改选中快递商",
                disabled: () => selectedCarrierKeys.length !== 1,
                onClick: () => openCarrierDialog(selectedCarrier),
              }}
              queryState={appliedQuery}
              querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)}
              onApplyQueryState={applyGridQueryState}
              onClearQueryState={clearGridQueryState}
          />
        </CardContent>
      </Card>

      <Card className="rounded-lg shadow-sm">
        <CardContent className="space-y-4 p-5">
          <SectionTitle icon={<Route className="size-4" aria-hidden />} title="快递选择规则" />
          <DataGrid
            storageKey="h5.express.routing-rules"
            columns={ruleColumns}
            data={rulesQuery.data ?? []}
            rowKey={(row) => row.id}
            selectable
            selectedRowKeys={selectedRuleKeys}
            onSelectedRowKeysChange={setSelectedRuleKeys}
            caption={rulesQuery.isPending ? "加载快递选择规则..." : undefined}
            emptyTitle="暂无快递选择规则"
            exportFileBaseName="H5 快递选择规则"
            tableClassName="min-w-[1050px]"
            refreshAction={{
              label: BUTTON_REFRESH,
              description: "刷新快递选择规则",
              disabled: rulesQuery.isFetching,
              onClick: () => void refreshRules(),
            }}
            createAction={{ label: BUTTON_ADD, description: "新增快递选择规则", onClick: () => openRuleDialog(null) }}
            editAction={{
              label: "修改",
              description: "修改选中规则",
              disabled: () => selectedRuleKeys.length !== 1,
              onClick: () => openRuleDialog(selectedRule),
            }}
            queryState={appliedQuery}
            querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)}
            onApplyQueryState={applyGridQueryState}
            onClearQueryState={clearGridQueryState}
          />
        </CardContent>
      </Card>

      <CarrierDialog
        open={carrierDialog.open}
        form={carrierDialog.target ?? emptyCarrierForm()}
        error={carrierDialog.open ? dialogError : null}
        saving={upsertCarrier.isPending}
        onFormChange={carrierDialog.setTarget}
        onOpenChange={carrierDialog.setOpen}
        onSave={saveCarrier}
      />
      <RuleDialog
        open={ruleDialog.open}
        form={ruleDialog.target ?? emptyRuleForm()}
        error={ruleDialog.open ? dialogError : null}
        saving={upsertRule.isPending}
        onFormChange={ruleDialog.setTarget}
        onOpenChange={ruleDialog.setOpen}
        onSave={saveRule}
      />
      <WaybillDialog
        open={waybillDialog.open}
        form={waybillDialog.target ?? emptyWaybillForm()}
        error={waybillDialog.open ? dialogError : null}
        saving={createWaybill.isPending}
        onFormChange={waybillDialog.setTarget}
        onOpenChange={waybillDialog.setOpen}
        onSave={saveWaybill}
      />
      <TrackingDialog
        open={trackingDialogOpen}
        waybill={recentWaybill}
        tracking={tracking}
        error={trackingDialogOpen ? dialogError : null}
        loading={trackingMutation.isPending}
        onOpenChange={setTrackingDialogOpen}
        onRefresh={() => recentWaybill && void loadTracking(recentWaybill.waybill_no)}
      />
      <CancelWaybillDialog
        open={cancelDialogOpen}
        waybill={recentWaybill}
        error={cancelDialogOpen ? dialogError : null}
        saving={cancelWaybill.isPending}
        onOpenChange={setCancelDialogOpen}
        onConfirm={() => recentWaybill && void cancelRecentWaybill(recentWaybill.waybill_no)}
      />
      <WaybillPrintDialog
        open={printDialogOpen}
        waybill={recentWaybill}
        onOpenChange={setPrintDialogOpen}
        onPrinted={printRecentWaybill}
      />
    </section>
  );

  async function refreshCarriers() {
    const result = await carriersQuery.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "快递商配置已刷新" });
  }

  async function refreshRules() {
    const result = await rulesQuery.refetch();
    setNotice(result.error ? { type: "error", text: result.error.message } : { type: "success", text: "快递选择规则已刷新" });
  }

  function applyGridQueryState(queryState: unknown) {
    applyQuery(queryValueFromUnknown(queryState));
  }

  function clearGridQueryState() {
    resetQuery();
  }

  function openCarrierDialog(carrier: ExpressCarrier | null) {
    setDialogError(null);
    carrierDialog.openWith(carrier ? carrierFormFromRow(carrier) : emptyCarrierForm());
  }

  function openRuleDialog(rule: ExpressRoutingRule | null) {
    setDialogError(null);
    ruleDialog.openWith(rule ? ruleFormFromRow(rule) : emptyRuleForm());
  }

  function openWaybillDialog(carrier: ExpressCarrier) {
    setDialogError(null);
    waybillDialog.openWith({ ...emptyWaybillForm(), carrierCode: carrier.carrier_code });
  }

  function openTrackingDialog() {
    setDialogError(null);
    setTrackingDialogOpen(true);
  }

  function openCancelDialog() {
    setDialogError(null);
    setCancelDialogOpen(true);
  }

  async function saveCarrier() {
    if (!carrierDialog.target) return;
    setDialogError(null);
    try {
      const saved = await upsertCarrier.mutateAsync(carrierRequestFromForm(carrierDialog.target));
      carrierDialog.close();
      setNotice({ type: "success", text: `${saved.carrier_name} 已保存` });
    } catch (errorValue) {
      setDialogError(errorText(errorValue, "保存快递商失败"));
    }
  }

  async function saveRule() {
    if (!ruleDialog.target) return;
    setDialogError(null);
    try {
      const saved = await upsertRule.mutateAsync(ruleRequestFromForm(ruleDialog.target));
      ruleDialog.close();
      setNotice({ type: "success", text: `${saved.rule_name} 已保存` });
    } catch (errorValue) {
      setDialogError(errorText(errorValue, "保存快递规则失败"));
    }
  }

  async function saveWaybill() {
    if (!waybillDialog.target) return;
    setDialogError(null);
    try {
      const created = await createWaybill.mutateAsync(waybillRequestFromForm(waybillDialog.target));
      setRecentWaybill(created);
      setTracking(null);
      waybillDialog.close();
      setNotice({ type: "success", text: `${created.waybill_no} 已生成` });
    } catch (errorValue) {
      setDialogError(errorText(errorValue, "快递下单失败"));
    }
  }

  async function loadTracking(waybillNo: string) {
    setDialogError(null);
    try {
      const response = await trackingMutation.mutateAsync(waybillNo);
      setTracking(response);
      setNotice({ type: "success", text: `${waybillNo} 轨迹已刷新` });
    } catch (errorValue) {
      setDialogError(errorText(errorValue, "查询轨迹失败"));
    }
  }

  async function cancelRecentWaybill(waybillNo: string) {
    setDialogError(null);
    try {
      const cancelled = await cancelWaybill.mutateAsync({ waybillNo, request: { reason: "管理端取消" } });
      setRecentWaybill(cancelled);
      setTracking(null);
      setCancelDialogOpen(false);
      setNotice({ type: "success", text: `${waybillNo} 已取消` });
    } catch (errorValue) {
      setDialogError(errorText(errorValue, "取消快递单失败"));
    }
  }

  function printRecentWaybill() {
    setPrintDialogOpen(false);
    setNotice({ type: "success", text: "面单预览已发送到浏览器打印" });
  }
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  return (
    <div className={`rounded-md border px-4 py-2 text-sm ${notice.type === "success" ? "border-emerald-200 bg-emerald-50 text-emerald-700" : "border-red-200 bg-red-50 text-red-700"}`}>
      {notice.text}
    </div>
  );
}

function SectionTitle({ icon, title }: { icon: React.ReactNode; title: string }) {
  return <h2 className="flex items-center gap-2 text-base font-semibold tracking-normal text-foreground">{icon}{title}</h2>;
}

function defaultQuery(): QueryPanelValue {
  return { q: "", enabled: [] };
}

function normalizeQuery(value: QueryPanelValue): QueryPanelValue {
  return { q: libQueryString(value.q).trim(), enabled: queryStringArray(value.enabled) };
}

function queryParams(value: QueryPanelValue) {
  const enabled = queryStringArray(value.enabled);
  return {
    q: libQueryString(value.q).trim(),
    enabled: enabled.length === 1 ? enabled[0] === "true" : undefined,
  };
}

// 演示预填仅在 dev server 生效（e2e 走 vite dev 依赖演示值）；生产构建表单一律为空，
// 避免顺丰 / 张三 / 13800000000 等虚构演示数据被原样保存成真实配置或运单。
function emptyCarrierForm(): CarrierForm {
  if (!__WMS_WEB_ADMIN_DEV_PREFILL__) {
    return {
      carrierCode: "",
      carrierName: "",
      apiUrl: "",
      apiKeyAlias: "",
      apiSecretAlias: "",
      accountNo: "",
      enabled: true,
      priority: "10",
      conditionsText: "{}",
    };
  }
  return {
    carrierCode: "SF",
    carrierName: "顺丰速运",
    apiUrl: "https://carrier.example.test/api",
    apiKeyAlias: "sf_api_key",
    apiSecretAlias: "sf_api_secret",
    accountNo: "WMS-001",
    enabled: true,
    priority: "10",
    conditionsText: JSON.stringify({ cold_chain: true }, null, 2),
  };
}

function carrierFormFromRow(row: ExpressCarrier): CarrierForm {
  return {
    carrierCode: row.carrier_code,
    carrierName: row.carrier_name,
    apiUrl: row.api_url,
    apiKeyAlias: row.api_key_alias ?? "",
    apiSecretAlias: row.api_secret_alias ?? "",
    accountNo: row.account_no ?? "",
    enabled: row.enabled,
    priority: String(row.priority),
    conditionsText: JSON.stringify(row.conditions ?? {}, null, 2),
  };
}

function emptyRuleForm(): RuleForm {
  if (!__WMS_WEB_ADMIN_DEV_PREFILL__) {
    return {
      ruleCode: "",
      ruleName: "",
      deliveryProviderType: "third_party_express",
      carrierCode: "",
      priority: "10",
      enabled: true,
      fallbackStrategy: "",
      conditionsText: "{}",
    };
  }
  return {
    ruleCode: "DEFAULT_THIRD_PARTY",
    ruleName: "默认三方快递",
    deliveryProviderType: "third_party_express",
    carrierCode: "SF",
    priority: "10",
    enabled: true,
    fallbackStrategy: "manual_review",
    conditionsText: JSON.stringify({ province: ["上海", "江苏", "浙江"] }, null, 2),
  };
}

function ruleFormFromRow(row: ExpressRoutingRule): RuleForm {
  return {
    ruleCode: row.rule_code,
    ruleName: row.rule_name,
    deliveryProviderType: row.delivery_provider_type === "own_fleet" ? "own_fleet" : "third_party_express",
    carrierCode: row.carrier_code ?? "",
    priority: String(row.priority),
    enabled: row.enabled,
    fallbackStrategy: row.fallback_strategy ?? "",
    conditionsText: JSON.stringify(row.conditions ?? {}, null, 2),
  };
}

function emptyWaybillForm(): WaybillForm {
  if (!__WMS_WEB_ADMIN_DEV_PREFILL__) {
    return {
      packageNo: "",
      carrierCode: "",
      senderName: "",
      senderMobile: "",
      senderAddress: "",
      receiverName: "",
      receiverMobile: "",
      receiverAddress: "",
      weightGrams: "",
      volumeCm3: "",
      packageCount: "1",
    };
  }
  return {
    packageNo: `PKG-${Date.now()}`,
    carrierCode: "SF",
    senderName: "平宇仓库",
    senderMobile: "13800000000",
    senderAddress: "上海市浦东新区 WMS 一号仓",
    receiverName: "张三",
    receiverMobile: "13900000000",
    receiverAddress: "上海市黄浦区客户门店",
    weightGrams: "1200",
    volumeCm3: "8000",
    packageCount: "1",
  };
}

function carrierRequestFromForm(form: CarrierForm): UpsertExpressCarrierRequest {
  return {
    carrier_code: form.carrierCode.trim(),
    carrier_name: form.carrierName.trim(),
    api_url: form.apiUrl.trim(),
    api_key_alias: optionalText(form.apiKeyAlias),
    api_secret_alias: optionalText(form.apiSecretAlias),
    account_no: optionalText(form.accountNo),
    enabled: form.enabled,
    priority: numberValue(form.priority, 100),
    conditions: parseJsonObject(form.conditionsText),
  };
}

function ruleRequestFromForm(form: RuleForm): UpsertExpressRoutingRuleRequest {
  return {
    rule_code: form.ruleCode.trim(),
    rule_name: form.ruleName.trim(),
    delivery_provider_type: form.deliveryProviderType,
    carrier_code: form.deliveryProviderType === "third_party_express" ? optionalText(form.carrierCode) : null,
    priority: numberValue(form.priority, 100),
    conditions: parseJsonObject(form.conditionsText),
    fallback_strategy: optionalText(form.fallbackStrategy),
    enabled: form.enabled,
    effective_from: null,
    effective_to: null,
  };
}

function waybillRequestFromForm(form: WaybillForm): CreateExpressWaybillRequest {
  return {
    outbound_order_id: null,
    package_no: form.packageNo.trim(),
    carrier_code: form.carrierCode.trim(),
    requested_waybill_no: null,
    sender_name: form.senderName.trim(),
    sender_mobile: form.senderMobile.trim(),
    sender_address: form.senderAddress.trim(),
    receiver_name: form.receiverName.trim(),
    receiver_mobile: form.receiverMobile.trim(),
    receiver_address: form.receiverAddress.trim(),
    weight_grams: numberValue(form.weightGrams, 1),
    volume_cm3: numberValue(form.volumeCm3, 0),
    package_count: numberValue(form.packageCount, 1),
  };
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function numberValue(value: string, fallback: number) {
  const next = Number(value);
  return Number.isFinite(next) ? next : fallback;
}

function parseJsonObject(value: string) {
  const parsed = JSON.parse(value || "{}") as unknown;
  if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") throw new Error("JSON 必须是对象");
  return parsed as Record<string, unknown>;
}
