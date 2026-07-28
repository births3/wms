import * as React from "react";
import {
  Button,
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { CalendarClock, FlaskConical, Power, Route, Upload } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  useCustomerAddressesQuery,
  useMasterDataRowsQuery,
} from "@/features/master-data/master-data-queries/queries";
import {
  useAggregationFieldsQuery,
  useAggregationRulesQuery,
  useCreateAggregationRuleDraftMutation,
  useCreateCutoffPlanMutation,
  useCutoffPlansQuery,
  useDeliveryNoteCandidatesQuery,
  useDeliveryNoteGroupsQuery,
  useDisableAggregationRuleMutation,
  useManualDeliveryNoteCutoffMutation,
  usePublishAggregationRuleMutation,
  usePublishCutoffPlanMutation,
  usePublishRouteBindingMutation,
  useRouteBindingsQuery,
  useTestAggregationRuleMutation,
  type AggregationRuleVersion,
  type CutoffPlan,
  type DeliveryNoteCandidate,
  type DeliveryNoteGroupListItem,
  type RouteBinding,
} from "@/features/print-orchestration/print-orchestration-queries";
import { formatDateTime } from "@/lib/format";
import { queryString, queryValueFromUnknown } from "@/lib/query-value";
import { usePageQueryState } from "@/lib/use-page-query-state";
import {
  AggregationRuleDialog,
  AggregationRuleTestDialog,
  CutoffPlanDialog,
  ManualCutoffDialog,
  RouteBindingDialog,
} from "./H9DeliveryNoteAggregationDialogs";
import {
  H9LifecycleConfirmDialog,
  type H9LifecycleConfirmation,
} from "./H9LifecycleConfirmDialog";
import { H9PrintSuitePanel } from "./H9PrintSuitePanel";

/**
 * 页面设计契约：列表/配置工作台；主信息为 QueryPanel + Tabs + DataGrid；
 * 人工截单、线路发布和截单计划维护使用 Dialog；详情、审计和任务执行不常驻本页。
 */

export const h9DeliveryNoteCoreQueryFieldKeys = ["warehouseId", "keyword"];

type AggregationLifecycleAction =
  | { kind: "publish-plan"; plan: CutoffPlan }
  | { kind: "publish-rule" | "disable-rule"; rule: AggregationRuleVersion };

export function H9DeliveryNoteAggregationPage({ currentUser }: { currentUser: CurrentUser }) {
  const warehousesQuery = useMasterDataRowsQuery("m1-warehouses");
  const partnersQuery = useMasterDataRowsQuery("m1-business-partners");
  const warehouses = warehousesQuery.data ?? [];
  const customers = (partnersQuery.data ?? []).filter((item) => item.partnerKind === "customer");
  const { draftQuery, setDraftQuery, appliedQuery, setAppliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(() => defaultQuery(), queryValueFromUnknown);
  const warehouseId = queryString(appliedQuery.warehouseId);
  const draftCustomerId = queryString(draftQuery.customerId);
  const addressesQuery = useCustomerAddressesQuery(draftCustomerId || null);
  const candidatesQuery = useDeliveryNoteCandidatesQuery(warehouseId);
  const groupsQuery = useDeliveryNoteGroupsQuery(warehouseId);
  const routesQuery = useRouteBindingsQuery(warehouseId);
  const plansQuery = useCutoffPlansQuery(warehouseId);
  const manualCutoff = useManualDeliveryNoteCutoffMutation();
  const publishRoute = usePublishRouteBindingMutation();
  const createPlan = useCreateCutoffPlanMutation();
  const publishPlan = usePublishCutoffPlanMutation();
  const aggregationFieldsQuery = useAggregationFieldsQuery();
  const rulesQuery = useAggregationRulesQuery();
  const createRule = useCreateAggregationRuleDraftMutation();
  const testRule = useTestAggregationRuleMutation();
  const publishRule = usePublishAggregationRuleMutation();
  const disableRule = useDisableAggregationRuleMutation();
  const [candidateIds, setCandidateIds] = React.useState<string[]>([]);
  const [planIds, setPlanIds] = React.useState<string[]>([]);
  const [ruleIds, setRuleIds] = React.useState<string[]>([]);
  const [manualOpen, setManualOpen] = React.useState(false);
  const [routeOpen, setRouteOpen] = React.useState(false);
  const [planOpen, setPlanOpen] = React.useState(false);
  const [ruleOpen, setRuleOpen] = React.useState(false);
  const [ruleTestOpen, setRuleTestOpen] = React.useState(false);
  const [lifecycleAction, setLifecycleAction] = React.useState<AggregationLifecycleAction | null>(null);
  const [notice, setNotice] = React.useState<string | null>(null);
  const canWrite = currentUser.permissions.includes("h9.print_orchestration.write");

  React.useEffect(() => {
    if (warehouses.length === 0 || warehouses.some((item) => item.id === queryString(draftQuery.warehouseId))) return;
    const next = { ...defaultQuery(), warehouseId: warehouses[0].id };
    setDraftQuery(next);
    setAppliedQuery(next);
  }, [draftQuery.warehouseId, setAppliedQuery, setDraftQuery, warehouses]);

  const warehouseOptions = React.useMemo(
    () => warehouses.map((item) => ({ value: item.id, label: `${item.code} ${item.name}` })),
    [warehouses],
  );
  const customerOptions = React.useMemo(
    () => customers.map((item) => ({ value: item.id, label: `${item.code} ${item.name}` })),
    [customers],
  );
  const addressOptions = React.useMemo(
    () => (addressesQuery.data ?? []).map((item) => ({
      value: item.id,
      label: `${item.province}${item.city}${item.district}${item.detail_address}`,
    })),
    [addressesQuery.data],
  );
  const h9DeliveryNoteQueryFields = React.useMemo(
    () => buildH9DeliveryNoteQueryFields(warehouseOptions, customerOptions, addressOptions),
    [addressOptions, customerOptions, warehouseOptions],
  );
  const candidates = React.useMemo(
    () => filterCandidates(candidatesQuery.data ?? [], appliedQuery),
    [appliedQuery, candidatesQuery.data],
  );
  const groups = React.useMemo(
    () => filterGroups(groupsQuery.data ?? [], appliedQuery),
    [appliedQuery, groupsQuery.data],
  );
  const routes = React.useMemo(
    () => filterRoutes(routesQuery.data ?? [], appliedQuery, customers),
    [appliedQuery, customers, routesQuery.data],
  );
  const plans = React.useMemo(
    () => filterPlans(plansQuery.data ?? [], appliedQuery, customers),
    [appliedQuery, customers, plansQuery.data],
  );
  const selectedCandidates = candidates.filter((item) => candidateIds.includes(item.outbound_order_id));
  const selectedPlan = plans.find((item) => planIds.includes(item.id)) ?? null;
  const rules = rulesQuery.data ?? [];
  const selectedRule = rules.find((item) => ruleIds.includes(item.id)) ?? null;
  const selectionValid = oneBoundary(selectedCandidates);
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h9DeliveryNoteQueryFields, appliedQuery),
    [appliedQuery, h9DeliveryNoteQueryFields],
  );
  const customerLabels = React.useMemo(
    () => new Map(customers.map((item) => [item.id, `${item.code} ${item.name}`])),
    [customers],
  );
  const warehouseLabels = React.useMemo(
    () => new Map(warehouses.map((item) => [item.id, `${item.code} ${item.name}`])),
    [warehouses],
  );

  const manualAction: DataGridToolbarAction = {
    key: "manual-cutoff",
    label: "人工截单",
    description: "将同一冻结边界下的已选订单归集",
    icon: <CalendarClock className="size-4" aria-hidden />,
    disabled: !canWrite || !selectionValid || manualCutoff.isPending,
    onClick: () => {
      manualCutoff.reset();
      setManualOpen(true);
    },
  };
  const routeCreateAction: DataGridCreateAction = {
    label: "发布线路",
    description: "发布送货地址的生效线路",
    disabled: !canWrite || publishRoute.isPending,
    onClick: () => {
      publishRoute.reset();
      setRouteOpen(true);
    },
  };
  const planCreateAction: DataGridCreateAction = {
    label: "新建计划",
    description: "新建结构化截单计划草稿",
    disabled: !canWrite || createPlan.isPending,
    onClick: () => {
      createPlan.reset();
      setPlanOpen(true);
    },
  };
  const publishPlanAction: DataGridToolbarAction = {
    key: "publish-plan",
    label: "发布计划",
    description: "发布选中的截单计划草稿",
    icon: <Route className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedPlan || selectedPlan.status !== "draft" || publishPlan.isPending,
    onClick: () => {
      if (!selectedPlan) return;
      publishPlan.reset();
      setLifecycleAction({ kind: "publish-plan", plan: selectedPlan });
    },
  };

  const ruleCreateAction: DataGridCreateAction = {
    label: "新建规则版本",
    description: "从已登记字段目录创建下一版归集规则草稿",
    disabled: !canWrite || createRule.isPending,
    onClick: () => {
      createRule.reset();
      setRuleOpen(true);
    },
  };
  const ruleTestAction: DataGridToolbarAction = {
    key: "test-rule",
    label: "测试规则",
    description: "用真实样本订单展示分组键与预计归集结果",
    icon: <FlaskConical className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedRule || !["draft", "tested"].includes(selectedRule.status) || testRule.isPending,
    onClick: () => {
      testRule.reset();
      setRuleTestOpen(true);
    },
  };
  const rulePublishAction: DataGridToolbarAction = {
    key: "publish-rule",
    label: "发布规则",
    description: "发布已测试版本；同货主旧发布版本自动停用",
    icon: <Upload className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedRule || selectedRule.status !== "tested" || publishRule.isPending,
    onClick: () => {
      if (!selectedRule) return;
      publishRule.reset();
      setLifecycleAction({ kind: "publish-rule", rule: selectedRule });
    },
  };
  const ruleDisableAction: DataGridToolbarAction = {
    key: "disable-rule",
    label: "停用规则",
    description: "停用当前发布版本，恢复仅按硬边界归集",
    icon: <Power className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedRule || selectedRule.status !== "published" || disableRule.isPending,
    onClick: () => {
      if (!selectedRule) return;
      disableRule.reset();
      setLifecycleAction({ kind: "disable-rule", rule: selectedRule });
    },
  };

  const lifecycleConfirmation: H9LifecycleConfirmation | null = lifecycleAction
    ? lifecycleAction.kind === "publish-plan"
      ? {
          title: "发布截单计划",
          description: `确认发布截单计划“${lifecycleAction.plan.name}”？发布后将参与自动截单。`,
          confirmLabel: "确认发布",
        }
      : {
          title: lifecycleAction.kind === "publish-rule" ? "发布归集规则" : "停用归集规则",
          description: lifecycleAction.kind === "publish-rule"
            ? `确认发布归集规则 V${lifecycleAction.rule.version_no}「${lifecycleAction.rule.name}」？同货主旧发布版本将自动停用。`
            : `确认停用归集规则 V${lifecycleAction.rule.version_no}「${lifecycleAction.rule.name}」？新订单将恢复按硬边界归集。`,
          confirmLabel: lifecycleAction.kind === "publish-rule" ? "确认发布" : "确认停用",
          destructive: lifecycleAction.kind === "disable-rule",
        }
    : null;

  async function confirmLifecycleAction() {
    if (!lifecycleAction) return;
    if (lifecycleAction.kind === "publish-plan") {
      const plan = lifecycleAction.plan;
      await publishPlan.mutateAsync(plan.id);
      setPlanIds([]);
      setLifecycleAction(null);
      setNotice(`截单计划“${plan.name}”已发布`);
      return;
    }
    const rule = lifecycleAction.kind === "publish-rule"
      ? await publishRule.mutateAsync(lifecycleAction.rule.id)
      : await disableRule.mutateAsync(lifecycleAction.rule.id);
    setRuleIds([]);
    setLifecycleAction(null);
    setNotice(`归集规则 V${rule.version_no} 已${lifecycleAction.kind === "publish-rule" ? "发布" : "停用"}`);
  }

  async function submitManual(reason: string) {
    const first = selectedCandidates[0];
    if (!first || !oneBoundary(selectedCandidates)) return;
    const group = await manualCutoff.mutateAsync({
      warehouse_id: first.warehouse_id,
      delivery_address_id: first.delivery_address_id,
      order_ids: selectedCandidates.map((item) => item.outbound_order_id),
      reason,
    });
    setCandidateIds([]);
    setManualOpen(false);
    setNotice(`已生成随货同行单号 ${group.delivery_note_no}`);
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="作业·随货同行单归集"
        subtitle="冻结送货线路，按客户优先、线路其次、货主加仓库兜底的截单计划归集订单"
        actions={notice ? <span className="self-center text-sm text-muted-foreground" role="status">{notice}</span> : undefined}
      />
      <QueryPanel
        fields={h9DeliveryNoteQueryFields}
        defaultVisibleFieldKeys={h9DeliveryNoteCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => {
          applyQuery(draftQuery);
          setCandidateIds([]);
          setPlanIds([]);
          setNotice("查询条件已应用");
        }}
        onReset={() => {
          resetQuery();
          setCandidateIds([]);
          setPlanIds([]);
        }}
      />
      <ErrorNotice message={
        warehousesQuery.error?.message
        ?? partnersQuery.error?.message
        ?? candidatesQuery.error?.message
        ?? groupsQuery.error?.message
        ?? routesQuery.error?.message
        ?? plansQuery.error?.message
        ?? publishPlan.error?.message
      } />
      {!canWrite && <p className="text-sm text-muted-foreground">当前账号仅可查看归集配置与结果。</p>}
      {candidateIds.length > 0 && !selectionValid && (
        <p className="text-sm text-destructive" role="alert">人工截单只能选择同一仓库、客户、送货地址和冻结线路的订单。</p>
      )}
      <Tabs defaultValue="candidates">
        <TabsList className="h-auto flex-wrap">
          <TabsTrigger value="candidates">待截单订单（{candidates.length}）</TabsTrigger>
          <TabsTrigger value="groups">截单结果（{groups.length}）</TabsTrigger>
          <TabsTrigger value="plans">截单计划（{plans.length}）</TabsTrigger>
          <TabsTrigger value="routes">线路绑定（{routes.length}）</TabsTrigger>
          <TabsTrigger value="rules">归集规则（{rules.length}）</TabsTrigger>
          <TabsTrigger value="print-suites">打印组套</TabsTrigger>
        </TabsList>
        <TabsContent value="candidates">
          <DataGrid
            columns={candidateColumns}
            data={candidates}
            rowKey={(row) => row.outbound_order_id}
            storageKey="h9-delivery-note-candidates"
            emptyTitle={warehouseId ? "暂无待截单订单" : "请选择仓库"}
            emptyDescription="仅展示已确认、已冻结线路且尚未归集的真实出库订单"
            caption={candidatesQuery.isPending ? "加载待截单订单..." : undefined}
            refreshAction={refreshAction(candidatesQuery, "待截单订单")}
            toolbarActions={[manualAction]}
            selectedRowKeys={candidateIds}
            onSelectedRowKeysChange={setCandidateIds}
            selectable
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={(value) => applyQuery(queryValueFromUnknown(value))}
            onClearQueryState={resetQuery}
            tableClassName="min-w-[1420px]"
          />
        </TabsContent>
        <TabsContent value="groups">
          <DataGrid
            columns={groupColumns}
            data={groups}
            rowKey={(row) => row.id}
            storageKey="h9-delivery-note-groups"
            emptyTitle="暂无截单结果"
            emptyDescription="人工或计划截单后将在这里展示"
            refreshAction={refreshAction(groupsQuery, "截单结果")}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            tableClassName="min-w-[1480px]"
          />
        </TabsContent>
        <TabsContent value="plans">
          <DataGrid
            columns={planColumns(customerLabels, warehouseLabels)}
            data={plans}
            rowKey={(row) => row.id}
            storageKey="h9.print-orchestration.cutoff-plans"
            emptyTitle="暂无截单计划"
            emptyDescription="新建计划草稿并发布后参与自动截单"
            refreshAction={refreshAction(plansQuery, "截单计划")}
            createAction={planCreateAction}
            toolbarActions={[publishPlanAction]}
            selectedRowKeys={planIds}
            onSelectedRowKeysChange={setPlanIds}
            selectable
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            tableClassName="min-w-[1320px]"
          />
        </TabsContent>
        <TabsContent value="routes">
          <DataGrid
            columns={routeColumns}
            data={routes}
            rowKey={(row) => row.id}
            storageKey="h9-route-bindings"
            emptyTitle="暂无线路绑定"
            emptyDescription="发布线路后，新订单进入时立即冻结线路"
            refreshAction={refreshAction(routesQuery, "线路绑定")}
            createAction={routeCreateAction}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            tableClassName="min-w-[1200px]"
          />
        </TabsContent>
        <TabsContent value="rules">
          <p className="mb-3 text-sm text-muted-foreground">
            维度只能从已登记订单标准字段中等值归组；货主 + 仓库 + 送货地址是不可覆盖的硬边界，规则不能跨地址归集。
          </p>
          <DataGrid
            columns={ruleColumns}
            data={rules}
            rowKey={(row) => row.id}
            storageKey="h9-aggregation-rules"
            emptyTitle="暂无归集规则版本"
            emptyDescription="未发布规则时按仓库 + 送货地址硬边界整体归集"
            caption={rulesQuery.isPending ? "加载归集规则..." : undefined}
            refreshAction={refreshAction(rulesQuery, "归集规则")}
            createAction={ruleCreateAction}
            toolbarActions={[ruleTestAction, rulePublishAction, ruleDisableAction]}
            selectedRowKeys={ruleIds}
            onSelectedRowKeysChange={(keys) => setRuleIds(keys.length ? [keys.at(-1) as string] : [])}
            selectable
            tableClassName="min-w-[1180px]"
          />
        </TabsContent>
        <TabsContent value="print-suites">
          <H9PrintSuitePanel
            canWrite={canWrite}
            warehouses={warehouseOptions}
            customers={customerOptions}
            groups={groupsQuery.data ?? []}
            onNotice={setNotice}
          />
        </TabsContent>
      </Tabs>
      <ManualCutoffDialog
        open={manualOpen}
        pending={manualCutoff.isPending}
        errorMessage={manualCutoff.error?.message}
        rows={selectedCandidates}
        onOpenChange={setManualOpen}
        onSubmit={submitManual}
      />
      <RouteBindingDialog
        open={routeOpen}
        pending={publishRoute.isPending}
        errorMessage={publishRoute.error?.message}
        warehouses={warehouseOptions}
        customers={customerOptions}
        onOpenChange={setRouteOpen}
        onSubmit={async (request) => {
          await publishRoute.mutateAsync(request);
          setRouteOpen(false);
          setNotice(`线路 ${request.route_code} 已发布`);
        }}
      />
      <CutoffPlanDialog
        open={planOpen}
        pending={createPlan.isPending}
        errorMessage={createPlan.error?.message}
        warehouses={warehouseOptions}
        customers={customerOptions}
        onOpenChange={setPlanOpen}
        onSubmit={async (request) => {
          await createPlan.mutateAsync(request);
          setPlanOpen(false);
          setNotice(`截单计划“${request.name}”草稿已保存`);
        }}
      />
      <AggregationRuleDialog
        open={ruleOpen}
        pending={createRule.isPending}
        errorMessage={createRule.error?.message ?? aggregationFieldsQuery.error?.message}
        fields={aggregationFieldsQuery.data ?? []}
        onOpenChange={setRuleOpen}
        onSubmit={async (request) => {
          const rule = await createRule.mutateAsync(request);
          setRuleOpen(false);
          setNotice(`归集规则草稿 V${rule.version_no}「${rule.name}」已保存`);
        }}
      />
      <AggregationRuleTestDialog
        open={ruleTestOpen}
        pending={testRule.isPending}
        errorMessage={testRule.error?.message}
        rule={selectedRule}
        candidates={candidates}
        result={testRule.data ?? null}
        onOpenChange={(next) => {
          setRuleTestOpen(next);
          if (!next) testRule.reset();
        }}
        onSubmit={async (orderIds) => {
          if (!selectedRule) return;
          const result = await testRule.mutateAsync({ versionId: selectedRule.id, orderIds });
          setNotice(`规则 V${result.rule.version_no} 测试完成：${result.groups.length} 组`);
        }}
      />
      <H9LifecycleConfirmDialog
        confirmation={lifecycleConfirmation}
        pending={publishPlan.isPending || publishRule.isPending || disableRule.isPending}
        errorMessage={
          lifecycleAction?.kind === "publish-plan"
            ? publishPlan.error?.message
            : lifecycleAction?.kind === "publish-rule"
              ? publishRule.error?.message
              : disableRule.error?.message
        }
        onOpenChange={(open) => {
          if (!open) setLifecycleAction(null);
        }}
        onConfirm={() => void confirmLifecycleAction().catch(() => undefined)}
      />
    </section>
  );
}

const ruleColumns: DataGridColumn<AggregationRuleVersion>[] = [
  { key: "version_no", header: "版本", width: 80, mono: true, render: (row) => `V${row.version_no}` },
  { key: "name", header: "规则名称", width: 220, render: (row) => row.name },
  { key: "status", header: "状态", width: 110, render: (row) => <StatusBadge status={ruleStatusKey(row.status)} label={ruleStatusLabel(row.status)} size="sm" /> },
  { key: "dimensions", header: "维度顺序（等值归组）", width: 340, render: (row) => row.dimensions.map((item) => aggregationFieldLabel(item.field_code)).join(" → ") },
  { key: "tested_at", header: "测试时间", width: 170, render: (row) => row.tested_at ? formatDateTime(row.tested_at) : "-" },
  { key: "published_at", header: "发布时间", width: 170, render: (row) => row.published_at ? formatDateTime(row.published_at) : "-" },
  { key: "created_at", header: "创建时间", width: 170, defaultHidden: true, render: (row) => formatDateTime(row.created_at) },
];

function ruleStatusKey(status: string) {
  return status === "published" ? "completed" : status === "disabled" ? "expired" : status === "tested" ? "in_progress" : "pending";
}

function ruleStatusLabel(status: string) {
  return status === "published" ? "已发布" : status === "disabled" ? "已停用" : status === "tested" ? "已测试" : "草稿";
}

export function aggregationFieldLabel(code: string) {
  const labels: Record<string, string> = {
    document_type: "单据类型",
    erp_order_no: "ERP 订单号",
    invoice_no: "发票号",
    transport_mode_code: "运输方式",
    department_code: "业务部门",
    sales_group_code: "销售组",
    order_group_no: "订单组号",
    business_type_code: "业务类型",
  };
  return labels[code] ?? code;
}

const candidateColumns: DataGridColumn<DeliveryNoteCandidate>[] = [
  { key: "wms_order_no", header: "WMS 订单号", width: 180, mono: true, copyValue: (row) => row.wms_order_no },
  { key: "erp_order_no", header: "ERP 订单号", width: 170, mono: true, render: (row) => row.erp_order_no ?? "-" },
  { key: "warehouse", header: "仓库", width: 160, render: (row) => `${row.warehouse_code} ${row.warehouse_name}` },
  { key: "customer", header: "客户", width: 180, render: (row) => `${row.customer_code} ${row.customer_name}` },
  { key: "delivery_address", header: "送货地址", width: 250, render: (row) => row.delivery_address },
  { key: "route_code", header: "冻结线路", width: 180, render: (row) => row.route_code },
  { key: "created_at", header: "订单创建时间", width: 160, render: (row) => formatDateTime(row.created_at) },
];

function buildH9DeliveryNoteQueryFields(
  warehouseOptions: Array<{ value: string; label: string }>,
  customerOptions: Array<{ value: string; label: string }>,
  addressOptions: Array<{ value: string; label: string }>,
): QueryPanelField[] {
  return [
    { key: "warehouseId", label: "仓库", type: "select", options: warehouseOptions },
    { key: "keyword", label: "关键字", type: "text", placeholder: "订单号 / 随货同行单号 / 客户" },
    { key: "customerId", label: "客户", type: "select", options: customerOptions },
    { key: "routeCode", label: "线路", type: "text", placeholder: "线路编码" },
    { key: "deliveryAddressId", label: "送货地址", type: "select", options: addressOptions },
  ];
}

const groupColumns: DataGridColumn<DeliveryNoteGroupListItem>[] = [
  { key: "delivery_note_no", header: "随货同行单号", width: 250, mono: true, copyValue: (row) => row.delivery_note_no },
  { key: "cutoff_mode", header: "截单方式", width: 100, render: (row) => <StatusBadge status={row.cutoff_mode === "manual" ? "pending" : "completed"} label={row.cutoff_mode === "manual" ? "人工截单" : "计划截单"} size="sm" /> },
  { key: "orders", header: "归集订单", width: 170, render: (row) => row.order_nos.join("、") },
  { key: "warehouse", header: "仓库", width: 180, defaultHidden: true, render: (row) => `${row.warehouse_code} ${row.warehouse_name}` },
  { key: "customer", header: "客户", width: 160, render: (row) => `${row.customer_code} ${row.customer_name}` },
  { key: "delivery_address", header: "送货地址", width: 240, render: (row) => row.delivery_address },
  { key: "route_code", header: "冻结线路", width: 150, render: (row) => row.route_code },
  { key: "cutoff_reason", header: "截单原因", width: 190, render: (row) => row.cutoff_reason ?? "按计划自动截单" },
  { key: "cutoff_at", header: "截单时间", width: 180, defaultHidden: true, render: (row) => formatDateTime(row.cutoff_at) },
];

function planColumns(customerLabels: Map<string, string>, warehouseLabels: Map<string, string>): DataGridColumn<CutoffPlan>[] {
  return [
    { key: "name", header: "计划名称", width: 200, render: (row) => row.name },
    { key: "status", header: "状态", width: 110, render: (row) => <StatusBadge status={row.status === "published" ? "completed" : "pending"} label={row.status === "published" ? "已发布" : row.status === "disabled" ? "已停用" : "草稿"} size="sm" /> },
    { key: "scope", header: "匹配层级", width: 130, render: (row) => scopeLabel(row.scope) },
    { key: "target", header: "匹配对象", width: 220, render: (row) => row.customer_id ? customerLabels.get(row.customer_id) ?? row.customer_id : row.route_code ?? "当前货主" },
    { key: "warehouse", header: "仓库", width: 180, defaultHidden: true, render: (row) => warehouseLabels.get(row.warehouse_id) ?? row.warehouse_id },
    { key: "weekly_schedule", header: "每周截单", width: 300, render: (row) => row.weekly_schedule.map((item) => `${weekdayLabel(item.weekday)} ${item.cutoff_time}`).join("；") },
    { key: "exceptions", header: "例外日期", width: 240, render: (row) => row.exceptions.length ? row.exceptions.map((item) => `${item.date} ${item.cutoff_time ?? "不截单"}`).join("；") : "无" },
    { key: "effective", header: "有效期", width: 330, defaultHidden: true, render: (row) => `${formatDateTime(row.effective_from)} 至 ${formatDateTime(row.effective_to)}` },
  ];
}

const routeColumns: DataGridColumn<RouteBinding>[] = [
  { key: "route_code", header: "线路编码", width: 160, mono: true, render: (row) => row.route_code },
  { key: "warehouse", header: "仓库", width: 210, render: (row) => `${row.warehouse_code} ${row.warehouse_name}` },
  { key: "customer", header: "客户", width: 220, render: (row) => `${row.customer_code} ${row.customer_name}` },
  { key: "delivery_address", header: "送货地址", width: 330, render: (row) => row.delivery_address },
  { key: "effective_from", header: "生效时间", width: 180, render: (row) => formatDateTime(row.effective_from) },
  { key: "effective_to", header: "失效时间", width: 180, render: (row) => formatDateTime(row.effective_to) },
];

function refreshAction(query: { isFetching: boolean; refetch: () => Promise<unknown> }, label: string): DataGridRefreshAction {
  return { label: "刷新", description: `刷新${label}`, disabled: query.isFetching, onClick: () => void query.refetch() };
}

function defaultQuery(): QueryPanelValue {
  return { warehouseId: "", keyword: "", customerId: "", routeCode: "", deliveryAddressId: "" };
}

function oneBoundary(rows: DeliveryNoteCandidate[]) {
  if (rows.length === 0) return false;
  const first = rows[0];
  return rows.every((row) =>
    row.warehouse_id === first.warehouse_id
    && row.customer_id === first.customer_id
    && row.delivery_address_id === first.delivery_address_id
    && row.route_code === first.route_code
  );
}

function filterCandidates(rows: DeliveryNoteCandidate[], query: QueryPanelValue) {
  return rows.filter((row) => matchesQuery(row, query, `${row.wms_order_no} ${row.erp_order_no ?? ""} ${row.customer_code} ${row.customer_name} ${row.delivery_address}`));
}

function filterGroups(rows: DeliveryNoteGroupListItem[], query: QueryPanelValue) {
  return rows.filter((row) => matchesQuery(row, query, `${row.delivery_note_no} ${row.order_nos.join(" ")} ${row.customer_code} ${row.customer_name} ${row.delivery_address}`));
}

function filterRoutes(rows: RouteBinding[], query: QueryPanelValue, customers: Array<{ id: string; code: string; name: string }>) {
  const labels = new Map(customers.map((item) => [item.id, `${item.code} ${item.name}`]));
  return rows.filter((row) => matchesQuery(row, query, `${row.route_code} ${labels.get(row.customer_id) ?? ""}`));
}

function filterPlans(rows: CutoffPlan[], query: QueryPanelValue, customers: Array<{ id: string; code: string; name: string }>) {
  const labels = new Map(customers.map((item) => [item.id, `${item.code} ${item.name}`]));
  return rows.filter((row) => matchesQuery(row, query, `${row.name} ${row.route_code ?? ""} ${row.customer_id ? labels.get(row.customer_id) ?? "" : ""}`));
}

function matchesQuery(
  row: { warehouse_id: string; customer_id?: string | null; route_code?: string | null; delivery_address_id?: string | null },
  query: QueryPanelValue,
  searchText: string,
) {
  const keyword = queryString(query.keyword).trim().toLocaleLowerCase("zh-CN");
  return (!queryString(query.warehouseId) || row.warehouse_id === queryString(query.warehouseId))
    && (!queryString(query.customerId) || row.customer_id === queryString(query.customerId))
    && (!queryString(query.routeCode) || (row.route_code ?? "").toLocaleLowerCase("zh-CN").includes(queryString(query.routeCode).trim().toLocaleLowerCase("zh-CN")))
    && (!queryString(query.deliveryAddressId) || row.delivery_address_id === queryString(query.deliveryAddressId))
    && (!keyword || searchText.toLocaleLowerCase("zh-CN").includes(keyword));
}

function scopeLabel(scope: CutoffPlan["scope"]) {
  return scope === "customer" ? "客户" : scope === "route" ? "线路" : "货主加仓库";
}

function weekdayLabel(weekday: number) {
  return ["", "周一", "周二", "周三", "周四", "周五", "周六", "周日"][weekday] ?? `星期${weekday}`;
}

function ErrorNotice({ message }: { message?: string }) {
  return message ? <p className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">{message}</p> : null;
}
