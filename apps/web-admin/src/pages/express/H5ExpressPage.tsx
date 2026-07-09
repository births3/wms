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
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { Ban, PackageCheck, Route, Truck } from "lucide-react";

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

type Notice = { type: "success" | "error"; text: string } | null;

const queryFields: QueryPanelField[] = [
  { key: "q", label: "关键字", type: "text", placeholder: "快递商 / 规则编码" },
  {
    key: "enabled",
    label: "启停状态",
    type: "multiSelect",
    options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }],
  },
];
const h5ExpressCoreQueryFieldKeys = ["q", "enabled"];

const providerOptions = [
  { label: "自有配送", value: "own_fleet" },
  { label: "三方快递", value: "third_party_express" },
];

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
    header: "状态",
    width: 110,
    minWidth: 90,
    filterValue: (row) => row.enabled ? "true" : "false",
    copyValue: (row) => row.enabled ? "启用" : "停用",
    filter: { type: "multiSelect", options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" />,
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
    header: "更新时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.updated_at,
    copyValue: (row) => formatDateTime(row.updated_at),
    filterValue: (row) => row.updated_at,
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.updated_at),
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filterValue: (row) => row.created_at,
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.created_at),
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
    header: "规则名称",
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
    header: "状态",
    width: 110,
    minWidth: 90,
    filterValue: (row) => row.enabled ? "true" : "false",
    copyValue: (row) => row.enabled ? "启用" : "停用",
    filter: { type: "multiSelect", options: [{ label: "启用", value: "true" }, { label: "停用", value: "false" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.enabled ? "启用" : "停用"} size="sm" />,
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
    header: "创建时间",
    width: 180,
    minWidth: 150,
    sortable: true,
    sortValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filterValue: (row) => row.created_at,
    filter: { type: "dateRange" },
    render: (row) => formatDateTime(row.created_at),
  },
];

export function H5ExpressPage() {
  const [query, setQuery] = React.useState<QueryPanelValue>(() => defaultQuery());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultQuery());
  const [selectedCarrierKeys, setSelectedCarrierKeys] = React.useState<string[]>([]);
  const [selectedRuleKeys, setSelectedRuleKeys] = React.useState<string[]>([]);
  const [carrierForm, setCarrierForm] = React.useState<CarrierForm>(() => emptyCarrierForm());
  const [ruleForm, setRuleForm] = React.useState<RuleForm>(() => emptyRuleForm());
  const [waybillForm, setWaybillForm] = React.useState<WaybillForm>(() => emptyWaybillForm());
  const [carrierDialogOpen, setCarrierDialogOpen] = React.useState(false);
  const [ruleDialogOpen, setRuleDialogOpen] = React.useState(false);
  const [waybillDialogOpen, setWaybillDialogOpen] = React.useState(false);
  const [trackingDialogOpen, setTrackingDialogOpen] = React.useState(false);
  const [cancelDialogOpen, setCancelDialogOpen] = React.useState(false);
  const [printDialogOpen, setPrintDialogOpen] = React.useState(false);
  const [notice, setNotice] = React.useState<Notice>(null);
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

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="H5 快递对接" subtitle="快递商配置、选择规则、快递下单、面单打印与轨迹查询" />
      <NoticePanel notice={notice} />
      <QueryPanel
        fields={queryFields}
        defaultVisibleFieldKeys={h5ExpressCoreQueryFieldKeys}
        value={query}
        onValueChange={(next) => setQuery(normalizeQuery(next))}
        onQuery={() => setAppliedQuery(normalizeQuery(query))}
        onReset={() => {
          const next = defaultQuery();
          setQuery(next);
          setAppliedQuery(next);
        }}
      />

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
                label: "刷新",
                description: "刷新快递商配置",
                disabled: carriersQuery.isFetching,
                onClick: () => void refreshCarriers(),
              }}
              createAction={{ label: "新增", description: "新增快递商", onClick: () => openCarrierDialog(null) }}
              editAction={{
                label: "修改",
                description: "修改选中快递商",
                disabled: () => selectedCarrierKeys.length !== 1,
                onClick: () => openCarrierDialog(selectedCarrier),
              }}
              printAction={{
                label: "打印",
                description: "打印最近生成的面单预览",
                disabled: () => !recentWaybill,
                onClick: () => setPrintDialogOpen(true),
              }}
              toolbarActions={[
                {
                  key: "waybill",
                  label: "下单",
                  description: "按选中快递商创建快递运单",
                  icon: <PackageCheck className="size-4" aria-hidden />,
                  disabled: () => !selectedCarrier || !selectedCarrier.enabled || createWaybill.isPending,
                  onClick: () => selectedCarrier && openWaybillDialog(selectedCarrier),
                },
                {
                  key: "tracking",
                  label: "轨迹",
                  description: "查询最近运单轨迹",
                  icon: <Route className="size-4" aria-hidden />,
                  disabled: () => !recentWaybill || trackingMutation.isPending,
                  onClick: () => setTrackingDialogOpen(true),
                },
                {
                  key: "cancel",
                  label: "取消",
                  description: "取消最近生成的快递单",
                  icon: <Ban className="size-4" aria-hidden />,
                  disabled: () => !recentWaybill || recentWaybill.status === "cancelled" || cancelWaybill.isPending,
                  onClick: () => setCancelDialogOpen(true),
                },
              ]}
              queryState={appliedQuery}
              querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)}
              onApplyQueryState={(queryState) => {
                const next = normalizeQuery(queryValueFromUnknown(queryState));
                setQuery(next);
                setAppliedQuery(next);
              }}
              onClearQueryState={() => {
                const next = defaultQuery();
                setQuery(next);
                setAppliedQuery(next);
              }}
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
              label: "刷新",
              description: "刷新快递选择规则",
              disabled: rulesQuery.isFetching,
              onClick: () => void refreshRules(),
            }}
            createAction={{ label: "新增", description: "新增快递选择规则", onClick: () => openRuleDialog(null) }}
            editAction={{
              label: "修改",
              description: "修改选中规则",
              disabled: () => selectedRuleKeys.length !== 1,
              onClick: () => openRuleDialog(selectedRule),
            }}
            queryState={appliedQuery}
            querySummaryItems={buildQueryPanelSummaryItems(queryFields, appliedQuery)}
            onApplyQueryState={(queryState) => {
              const next = normalizeQuery(queryValueFromUnknown(queryState));
              setQuery(next);
              setAppliedQuery(next);
            }}
            onClearQueryState={() => {
              const next = defaultQuery();
              setQuery(next);
              setAppliedQuery(next);
            }}
          />
        </CardContent>
      </Card>

      <CarrierDialog
        open={carrierDialogOpen}
        form={carrierForm}
        saving={upsertCarrier.isPending}
        onFormChange={setCarrierForm}
        onOpenChange={setCarrierDialogOpen}
        onSave={saveCarrier}
      />
      <RuleDialog
        open={ruleDialogOpen}
        form={ruleForm}
        saving={upsertRule.isPending}
        onFormChange={setRuleForm}
        onOpenChange={setRuleDialogOpen}
        onSave={saveRule}
      />
      <WaybillDialog
        open={waybillDialogOpen}
        form={waybillForm}
        saving={createWaybill.isPending}
        onFormChange={setWaybillForm}
        onOpenChange={setWaybillDialogOpen}
        onSave={saveWaybill}
      />
      <TrackingDialog
        open={trackingDialogOpen}
        waybill={recentWaybill}
        tracking={tracking}
        loading={trackingMutation.isPending}
        onOpenChange={setTrackingDialogOpen}
        onRefresh={() => recentWaybill && void loadTracking(recentWaybill.waybill_no)}
      />
      <CancelWaybillDialog
        open={cancelDialogOpen}
        waybill={recentWaybill}
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

  function openCarrierDialog(carrier: ExpressCarrier | null) {
    setCarrierForm(carrier ? carrierFormFromRow(carrier) : emptyCarrierForm());
    setCarrierDialogOpen(true);
  }

  function openRuleDialog(rule: ExpressRoutingRule | null) {
    setRuleForm(rule ? ruleFormFromRow(rule) : emptyRuleForm());
    setRuleDialogOpen(true);
  }

  function openWaybillDialog(carrier: ExpressCarrier) {
    setWaybillForm({ ...emptyWaybillForm(), carrierCode: carrier.carrier_code });
    setWaybillDialogOpen(true);
  }

  async function saveCarrier() {
    try {
      const saved = await upsertCarrier.mutateAsync(carrierRequestFromForm(carrierForm));
      setCarrierDialogOpen(false);
      setNotice({ type: "success", text: `${saved.carrier_name} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存快递商失败") });
    }
  }

  async function saveRule() {
    try {
      const saved = await upsertRule.mutateAsync(ruleRequestFromForm(ruleForm));
      setRuleDialogOpen(false);
      setNotice({ type: "success", text: `${saved.rule_name} 已保存` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "保存快递规则失败") });
    }
  }

  async function saveWaybill() {
    try {
      const created = await createWaybill.mutateAsync(waybillRequestFromForm(waybillForm));
      setRecentWaybill(created);
      setTracking(null);
      setWaybillDialogOpen(false);
      setNotice({ type: "success", text: `${created.waybill_no} 已生成` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "快递下单失败") });
    }
  }

  async function loadTracking(waybillNo: string) {
    try {
      const response = await trackingMutation.mutateAsync(waybillNo);
      setTracking(response);
      setNotice({ type: "success", text: `${waybillNo} 轨迹已刷新` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "查询轨迹失败") });
    }
  }

  async function cancelRecentWaybill(waybillNo: string) {
    try {
      const cancelled = await cancelWaybill.mutateAsync({ waybillNo, request: { reason: "管理端取消" } });
      setRecentWaybill(cancelled);
      setTracking(null);
      setCancelDialogOpen(false);
      setNotice({ type: "success", text: `${waybillNo} 已取消` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorText(errorValue, "取消快递单失败") });
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
  return { q: queryString(value.q), enabled: queryStringArray(value.enabled) };
}

function queryParams(value: QueryPanelValue) {
  const enabled = queryStringArray(value.enabled);
  return {
    q: queryString(value.q),
    enabled: enabled.length === 1 ? enabled[0] === "true" : undefined,
  };
}

function emptyCarrierForm(): CarrierForm {
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

function providerLabel(value: string) {
  return value === "own_fleet" ? "自有配送" : "三方快递";
}

function queryString(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function queryStringArray(value: unknown) {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" ? value as QueryPanelValue : defaultQuery();
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

function formatDateTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function errorText(errorValue: unknown, fallback: string) {
  return errorValue instanceof Error ? errorValue.message : fallback;
}
