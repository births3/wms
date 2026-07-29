import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  buildQueryPanelSummaryItems,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDetailAction,
  type DataGridExportAction,
  type DataGridPrintAction,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import {
  M4OutboundDetailDialog,
  statusLabel,
  type DetailTarget,
  type OutboundOrder,
  type OutboundWave,
  type PurchaseReturnOrder,
} from "./M4OutboundDetailDialog";
import {
  M4OutboundActionDialog,
  outboundPrivateActions,
  type ActionKind,
  type ActionState,
  type ActionTargetContext,
  type OutboundCreateForm,
} from "./M4OutboundActionDialog";
import {
  useCancelOutboundWaveMutation,
  useCreateOutboundOrderMutation,
  useCreateOutboundWaveMutation,
  useOutboundOrderQuery,
  useOutboundOrdersQuery,
  useOutboundReviewQuery,
  useOutboundWaveQuery,
  useOutboundWavesQuery,
  useReviewOutboundOrderMutation,
  useShipOutboundOrderMutation,
  type CreateOutboundOrderRequest,
  type CreateOutboundWaveRequest,
} from "@/features/outbound/outbound-queries";
import { useCurrentUserQuery } from "@/features/auth/auth-queries";
import {
  useCustomerAddressesQuery,
  useMasterDataRowsQuery,
  useSystemDictionaryItemOptionsQuery,
} from "@/features/master-data/master-data-queries";
import {
  useDualPersonPolicyQueries,
} from "@/features/validation-rules/dual-person-policy-queries";
import {
  BatchNoCell,
  CustomerCell,
  OrderNoSummary,
  ProductSummary,
  ReviewSummary,
  TwoLine,
  ValidationBadge,
  purchaseReturnDocumentTypeLabel,
} from "./M4OutboundPageParts";
import {
  defaultM4OutboundQueryValue,
  filterOrders,
  filterReturns,
  filterWaves,
  normalizeM4OutboundQueryValue,
  queryValueFromUnknown,
  pageMeta,
  statusKey,
  statusOptions,
  type M4OutboundMode,
} from "./m4-outbound-page-model";
import {
  buildShipOutboundRequest,
  defaultOutboundShipForm,
  formatDate,
  isUuid,
  makeOrder,
  type OutboundShipForm,
  makeReturn,
  outboundCustomerId as customerId,
  outboundOwnerId as ownerId,
  outboundWarehouseId as warehouseId,
  strictestDualPersonPolicy,
  toInteger,
  waveLineCount,
  waveQty,
} from "./m4-outbound-page-helpers";

export type { M4OutboundMode } from "./m4-outbound-page-model";

interface M4OutboundPageProps {
  mode: M4OutboundMode;
  onBack: () => void;
}

const m4OutboundStatusOptions = [
  { value: "draft", label: "草稿" },
  { value: "pending_validation", label: "待校验" },
  { value: "validation_exception", label: "校验异常" },
  { value: "confirmed", label: "已确认" },
  { value: "void_requested", label: "作废申请中" },
  { value: "in_wave", label: "已进波次" },
  { value: "inventory_locked", label: "库存锁定" },
  { value: "picked", label: "已拣选" },
  { value: "picked_short", label: "短拣待补齐" },
  { value: "released", label: "已下发" },
  { value: "reviewed", label: "已复核" },
  { value: "reviewed_short", label: "短拣已复核" },
  { value: "shipped", label: "已发货" },
  { value: "pending_approval", label: "待审批" },
  { value: "approved", label: "已审批" },
  { value: "picking", label: "拣货中" },
  { value: "cancelled", label: "已取消" },
];

const m4OutboundQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text", placeholder: "单号 / 商品 / 批号 / 客商" },
  { key: "statusFilter", label: "状态", type: "multiSelect", options: m4OutboundStatusOptions },
  { key: "businessDate", label: "业务日期", type: "dateRange" },
];
const m4OutboundCoreQueryFieldKeys = ["keyword", "statusFilter"];

// ponytail: local state mirrors successful API responses so the grid updates without inventing a success result.
const seedOrders: OutboundOrder[] = [
  makeOrder("00000000-0000-0000-0000-000000006001", "SO-M4-PC-0001", "ERP-SO-0001", "confirmed", 36, false),
  makeOrder("00000000-0000-0000-0000-000000006002", "SO-M4-PC-0002", "ERP-SO-0002", "inventory_locked", 18, true),
  makeOrder("00000000-0000-0000-0000-000000006003", "SO-M4-PC-0003", "ERP-SO-0003", "reviewed", 24, false),
];

const seedWaves: OutboundWave[] = [
  {
    id: "00000000-0000-0000-0000-000000007001",
    owner_id: ownerId,
    wave_no: "WAVE-M4-PC-0001",
    status: "released",
    order_ids: [seedOrders[1].id],
    created_at: "2026-06-27T08:30:00.000Z",
    updated_at: "2026-06-27T08:45:00.000Z",
  },
];

const seedReturns: PurchaseReturnOrder[] = [
  {
    id: "00000000-0000-0000-0000-000000008001",
    return_no: "PRTN-M4-PC-0001",
    document_type: "purchase_return_outbound",
    source_purchase_order_no: "ASN-M2-PC-0001",
    supplier_name: "华东医药供应商",
    reason: "供应商召回",
    approval_source: "purchase_return_approval",
    status: "pending_approval",
    product_code: "P-M4-001",
    qty: 6,
    created_at: "2026-06-27T10:00:00.000Z",
    updated_at: "2026-06-27T10:00:00.000Z",
  },
];

export function M4OutboundPage({ mode }: M4OutboundPageProps) {
  const [orders, setOrders] = React.useState<OutboundOrder[]>(seedOrders);
  const [waves, setWaves] = React.useState<OutboundWave[]>(seedWaves);
  const [returns, setReturns] = React.useState<PurchaseReturnOrder[]>(seedReturns);
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultM4OutboundQueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultM4OutboundQueryValue());
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [detailTarget, setDetailTarget] = React.useState<DetailTarget | null>(null);
  const [activeAction, setActiveAction] = React.useState<ActionState | null>(null);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [note, setNote] = React.useState("");
  const [actionError, setActionError] = React.useState<string | null>(null);
  const [secondReviewerId, setSecondReviewerId] = React.useState("");
  const [createForm, setCreateForm] = React.useState<OutboundCreateForm>({
    wmsOrderNo: "",
    erpOrderNo: "ERP-SO-NEW",
    documentType: "sales_outbound",
    deliveryAddressId: "",
    customerName: "连锁门店 A",
    productCode: "P-M4-NEW",
    batchNo: "BATCH-OUT-202607",
    plannedQty: "12",
    requiredShipDate: "",
  });
  const [shipForm, setShipForm] = React.useState<OutboundShipForm>(defaultOutboundShipForm);
  const ordersQuery = useOutboundOrdersQuery(mode === "orders" || mode === "waves" || mode === "review");
  const wavesQuery = useOutboundWavesQuery(mode === "waves");
  const createOutboundOrderMutation = useCreateOutboundOrderMutation();
  const createOutboundWaveMutation = useCreateOutboundWaveMutation();
  const cancelOutboundWaveMutation = useCancelOutboundWaveMutation();
  const reviewOutboundOrderMutation = useReviewOutboundOrderMutation();
  const shipOutboundOrderMutation = useShipOutboundOrderMutation();
  const currentUserQuery = useCurrentUserQuery(true);
  const orderDetailQuery = useOutboundOrderQuery(detailTarget?.kind === "order" ? detailTarget.value.id : null);
  const waveDetailQuery = useOutboundWaveQuery(detailTarget?.kind === "wave" ? detailTarget.value.id : null);
  const reviewDetailQuery = useOutboundReviewQuery(activeAction?.kind === "review" ? activeAction.targetId ?? null : null);
  const reviewProductsQuery = useMasterDataRowsQuery("m1-products", activeAction?.kind === "review");
  const reviewPolicyInputs = React.useMemo(() => {
    if (activeAction?.kind !== "review" || !reviewDetailQuery.data || !reviewProductsQuery.data) return [];
    const productsByCode = new Map(reviewProductsQuery.data.map((item) => [item.code, item]));
    const productIds = new Set<string>();
    for (const line of reviewDetailQuery.data.lines ?? []) {
      const product = productsByCode.get(line.product_code);
      if (!product) return [];
      productIds.add(product.id);
    }
    return [...productIds].map((productId) => ({
      productId,
      process: "出库",
      node: "复核",
      ownerId: reviewDetailQuery.data!.owner_id,
      warehouseId: reviewDetailQuery.data!.warehouse_id,
    }));
  }, [activeAction?.kind, reviewDetailQuery.data, reviewProductsQuery.data]);
  const reviewPolicyQueries = useDualPersonPolicyQueries(reviewPolicyInputs);
  const reviewPolicyLoading = reviewProductsQuery.isPending
    || reviewPolicyQueries.some((query) => query.isFetching);
  const reviewPolicy = reviewPolicyInputs.length > 0
    && reviewPolicyQueries.every((query) => query.data)
    ? strictestDualPersonPolicy(reviewPolicyQueries.map((query) => query.data!.policy))
    : null;
  const reviewNeedsSecond = reviewPolicy === "dual_scan" || reviewPolicy === "dual_scan_with_approval";
  const documentTypeOptionsQuery = useSystemDictionaryItemOptionsQuery("document_type", mode === "orders");
  const deliveryAddressesQuery = useCustomerAddressesQuery(mode === "orders" ? customerId : null);
  const deliveryAddressOptions = React.useMemo(
    () => (deliveryAddressesQuery.data ?? []).map((address) => ({
      value: address.id,
      label: `${address.province}${address.city}${address.district}${address.detail_address} · ${address.contact_name}`,
    })),
    [deliveryAddressesQuery.data],
  );
  const documentTypeOptions = React.useMemo(
    () => (documentTypeOptionsQuery.data ?? [])
      .filter(([value]) => value === "sales_outbound" || value === "purchase_return_outbound")
      .map(([value, label]) => ({ value, label })),
    [documentTypeOptionsQuery.data],
  );
  const renderedDetailTarget = React.useMemo<DetailTarget | null>(() => {
    if (detailTarget?.kind === "order" && orderDetailQuery.data) {
      return { kind: "order", value: orderDetailQuery.data };
    }
    if (detailTarget?.kind === "wave" && waveDetailQuery.data) {
      return {
        kind: "wave",
        value: waveDetailQuery.data,
        orders: orders.filter((order) => (waveDetailQuery.data.order_ids ?? []).includes(order.id)),
      };
    }
    return detailTarget;
  }, [detailTarget, orderDetailQuery.data, orders, waveDetailQuery.data]);

  React.useEffect(() => {
    setSelectedId(null);
    const next = defaultM4OutboundQueryValue();
    setDraftQuery(next);
    setAppliedQuery(next);
    setLastEvent(null);
  }, [mode]);

  React.useEffect(() => {
    if (!ordersQuery.isPending && !ordersQuery.error && ordersQuery.data) setOrders(ordersQuery.data);
  }, [ordersQuery.data, ordersQuery.error, ordersQuery.isPending]);

  React.useEffect(() => {
    if (!wavesQuery.isPending && !wavesQuery.error && wavesQuery.data) setWaves(wavesQuery.data);
  }, [wavesQuery.data, wavesQuery.error, wavesQuery.isPending]);

  React.useEffect(() => {
    const firstAddressId = deliveryAddressesQuery.data?.[0]?.id;
    if (!firstAddressId) return;
    setCreateForm((value) => value.deliveryAddressId
      ? value
      : { ...value, deliveryAddressId: firstAddressId });
  }, [deliveryAddressesQuery.data]);

  React.useEffect(() => {
    if (!lastEvent) return;
    const timer = window.setTimeout(() => setLastEvent(null), 3000);
    return () => window.clearTimeout(timer);
  }, [lastEvent]);

  const meta = pageMeta(mode);
  const normalizedQuery = normalizeM4OutboundQueryValue(appliedQuery);
  const filteredOrders = filterOrders(orders, normalizedQuery, mode);
  const filteredWaves = filterWaves(waves, normalizedQuery);
  const filteredReturns = filterReturns(returns, normalizedQuery);
  const selectedOrder = filteredOrders.find((item) => item.id === selectedId) ?? null;
  const selectedWave = filteredWaves.find((item) => item.id === selectedId) ?? null;
  const selectedReturn = filteredReturns.find((item) => item.id === selectedId) ?? null;
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m4OutboundQueryFields, appliedQuery),
    [appliedQuery],
  );
  const createActionKind = meta.createAction;
  const gridRefreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: `刷新${meta.title}列表`,
    disabled: ordersQuery.isFetching || wavesQuery.isFetching,
    onClick: () => {
      void refreshOutbound();
    },
  };
  const gridCreateAction: DataGridCreateAction | undefined = createActionKind
    ? {
        label: "新增",
        description: meta.createLabel,
        onClick: () => openAction(createActionKind),
      }
    : undefined;
  const gridDetailAction: DataGridDetailAction = {
    label: "详情",
    description: "查看选中单据详情",
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => openSelectedDetail(selectedRowKeys[0]),
  };
  const gridPrintAction: DataGridPrintAction = {
    label: "打印",
    description: `打印${meta.title}`,
    disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
    onClick: ({ selectedRowKeys }) => {
      const targetId = selectedRowKeys[0];
      if (targetId) openAction("print", targetId);
    },
  };
  const gridExportAction: DataGridExportAction = {
    label: "导出",
    description: `导出${meta.title}`,
  };
  const gridToolbarActions = outboundPrivateActions(mode, selectedOrder, selectedWave, selectedReturn, openAction);

  const orderColumns: DataGridColumn<OutboundOrder>[] = mode === "review"
    ? [
        { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 220, width: 240, onDoubleClick: (row) => openOrderDetail(row.id), render: (row) => <OrderNoSummary order={row} /> },
        { key: "product", header: "计划件数", minWidth: 120, render: (row) => <ReviewSummary order={row} /> },
        { key: "customer_id", header: "客户 / 门店", minWidth: 160, render: (row) => <CustomerCell customerId={row.customer_id} /> },
        { key: "required_ship_at", header: "要求发货", minWidth: 150, render: (row) => formatDate(row.required_ship_at) },
        { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
        { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
      ]
    : [
        { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 220, width: 240, onDoubleClick: (row) => openOrderDetail(row.id), render: (row) => <OrderNoSummary order={row} /> },
        { key: "product", header: "商品 / 数量", minWidth: 140, render: (row) => <ProductSummary order={row} /> },
        { key: "batch_no", header: "批号", mono: true, minWidth: 150, render: (row) => <BatchNoCell order={row} /> },
        { key: "validation", header: "校验", minWidth: 100, render: (row) => <ValidationBadge order={row} /> },
        { key: "customer_id", header: "客户 / 门店", minWidth: 150, render: (row) => <CustomerCell customerId={row.customer_id} /> },
        { key: "required_ship_at", header: "要求发货", minWidth: 150, render: (row) => formatDate(row.required_ship_at) },
        { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
        { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
      ];

  const waveColumns: DataGridColumn<OutboundWave>[] = [
    { key: "wave_no", header: "波次号", mono: true, minWidth: 180, onDoubleClick: (row) => openWaveDetail(row.id), render: (row) => <span className="text-primary">{row.wave_no}</span> },
    { key: "orders", header: "订单 / 明细", minWidth: 140, render: (row) => `${(row.order_ids ?? []).length} 单 / ${waveLineCount(row, orders)} 行` },
    { key: "qty", header: "件数 / 温区", align: "right", minWidth: 130, render: (row) => `${waveQty(row, orders)} 件 / 常温` },
    { key: "route", header: "路径策略 / 容量", minWidth: 180, render: () => <TwoLine top="S 型最短路径" bottom="容量上限 100 单 / 10000 件" /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
  ];

  const returnColumns: DataGridColumn<PurchaseReturnOrder>[] = [
    { key: "return_no", header: "采购退货单 / 类型", mono: true, minWidth: 240, width: 260, onDoubleClick: (row) => openReturnDetail(row.id), render: (row) => <TwoLine top={row.return_no} bottom={purchaseReturnDocumentTypeLabel(row.document_type)} /> },
    { key: "source_purchase_order_no", header: "原采购入库单", mono: true, minWidth: 180 },
    { key: "supplier_name", header: "供应商 / 原因", minWidth: 200, render: (row) => <TwoLine top={row.supplier_name} bottom={row.reason} /> },
    { key: "product", header: "商品 / 数量", minWidth: 160, render: (row) => <TwoLine top={row.product_code} bottom={`${row.qty} 件`} /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
  ];

  async function refreshOutbound() {
    if (mode === "orders" || mode === "review") await ordersQuery.refetch();
    if (mode === "waves") await Promise.all([ordersQuery.refetch(), wavesQuery.refetch()]);
    setLastEvent(`${meta.title}已刷新`);
  }

  function openSelectedDetail(id?: string) {
    if (!id) return;
    if (mode === "waves") {
      openWaveDetail(id);
      return;
    }
    if (mode === "returns") {
      openReturnDetail(id);
      return;
    }
    openOrderDetail(id);
  }

  function openOrderDetail(id: string) {
    const order = orders.find((item) => item.id === id);
    if (!order) return;
    setSelectedId(id);
    setDetailTarget({ kind: "order", value: order });
    setDetailOpen(true);
  }

  function openWaveDetail(id: string) {
    const wave = waves.find((item) => item.id === id);
    if (!wave) return;
    setSelectedId(id);
    setDetailTarget({ kind: "wave", value: wave, orders: orders.filter((order) => (wave.order_ids ?? []).includes(order.id)) });
    setDetailOpen(true);
  }

  function openReturnDetail(id: string) {
    const returnOrder = returns.find((item) => item.id === id);
    if (!returnOrder) return;
    setSelectedId(id);
    setDetailTarget({ kind: "return", value: returnOrder });
    setDetailOpen(true);
  }

  function openAction(kind: ActionKind, targetId?: string) {
    if (kind === "create-order") createOutboundOrderMutation.reset();
    if (kind === "create-wave") createOutboundWaveMutation.reset();
    if (kind === "cancel-wave") cancelOutboundWaveMutation.reset();
    if (kind === "review") reviewOutboundOrderMutation.reset();
    if (kind === "ship") { shipOutboundOrderMutation.reset(); setShipForm(defaultOutboundShipForm()); }
    setActiveAction({ kind, targetId });
    setNote("");
    setSecondReviewerId("");
    setActionError(null);
  }

  function resolveActionTarget(action: ActionState | null): ActionTargetContext | null {
    if (!action?.targetId) return null;
    const order = orders.find((item) => item.id === action.targetId);
    if (order) {
      return {
        id: order.id,
        docNo: order.wms_order_no,
        status: order.status,
        statusText: statusLabel(order.status),
        kindLabel: "出库单",
      };
    }
    const wave = waves.find((item) => item.id === action.targetId);
    if (wave) {
      return {
        id: wave.id,
        docNo: wave.wave_no,
        status: wave.status,
        statusText: statusLabel(wave.status),
        kindLabel: "波次",
      };
    }
    const returnOrder = returns.find((item) => item.id === action.targetId);
    if (returnOrder) {
      return {
        id: returnOrder.id,
        docNo: returnOrder.return_no,
        status: returnOrder.status,
        statusText: statusLabel(returnOrder.status),
        kindLabel: "采购退货单",
      };
    }
    return null;
  }

  async function submitAction(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!activeAction) return;
    if (activeAction.kind === "reject-return" && !note.trim()) {
      setActionError("驳回备注必填");
      return;
    }
    try {
      await applyAction(activeAction);
      setActiveAction(null);
      setActionError(null);
    } catch (error) {
      setActionError(error instanceof Error ? error.message : "操作失败，请稍后重试");
    }
  }

  async function applyAction(action: ActionState) {
    if (action.kind === "create-order") {
      const request: CreateOutboundOrderRequest = {
        document_type: createForm.documentType,
        wms_order_no: createForm.wmsOrderNo.trim(),
        erp_order_no: createForm.erpOrderNo.trim() || null,
        customer_id: customerId,
        delivery_address_id: createForm.deliveryAddressId,
        warehouse_id: warehouseId,
        required_ship_at: createForm.requiredShipDate ? `${createForm.requiredShipDate}T09:00:00.000Z` : null,
        lines: [{
          line_no: 1,
          product_code: createForm.productCode.trim(),
          batch_no: createForm.batchNo.trim(),
          planned_qty: toInteger(createForm.plannedQty),
        }],
      };
      const created = await createOutboundOrderMutation.mutateAsync(request);
      setOrders((value) => [created, ...value]);
      setSelectedId(created.id);
      setLastEvent(`${created.wms_order_no} 已创建`);
      return;
    }
    if (action.kind === "create-wave") {
      const orderIds = orders.filter((order) => order.status === "confirmed").map((order) => order.id).slice(0, 2);
      if (orderIds.length === 0) throw new Error("没有可加入波次的已确认出库单");
      const request: CreateOutboundWaveRequest = {
        wave_no: `WAVE-M4-PC-${Date.now()}`,
        order_ids: orderIds,
      };
      const wave = await createOutboundWaveMutation.mutateAsync(request);
      setWaves((value) => [wave, ...value]);
      setOrders((value) => value.map((order) => orderIds.includes(order.id) ? { ...order, status: "in_wave" } : order));
      setLastEvent(`${wave.wave_no} 已创建`);
      return;
    }
    if (action.kind === "cancel-wave") {
      if (!action.targetId) return;
      const currentWave = waves.find((wave) => wave.id === action.targetId);
      const cancelled = await cancelOutboundWaveMutation.mutateAsync(action.targetId);
      setWaves((value) => value.map((wave) => wave.id === cancelled.id ? cancelled : wave));
      if (currentWave) {
        const orderIds = currentWave.order_ids ?? [];
        setOrders((value) => value.map((order) => orderIds.includes(order.id) ? { ...order, status: "confirmed" } : order));
      }
      setLastEvent(`${cancelled.wave_no} 已取消`);
      return;
    }
    if (action.kind === "create-return") {
      const item = makeReturn(`RTN-M4-PC-${Date.now()}`);
      setReturns((value) => [item, ...value]);
      setLastEvent(`${item.return_no} 已创建`);
      return;
    }
    if (!action.targetId) return;
    if (action.kind === "validate") updateOrder(action.targetId, "confirmed", "重新校验已完成");
    if (action.kind === "void") updateOrder(action.targetId, "void_requested", "作废申请已提交");
    if (action.kind === "review") {
      const order = reviewDetailQuery.data;
      const reviewerId = currentUserQuery.data?.user_id;
      if (!order) throw new Error(reviewDetailQuery.error?.message ?? "复核明细尚未读取完成");
      if (!reviewerId) throw new Error("当前登录用户信息不可用，无法提交复核");
      const normalizedSecondReviewerId = secondReviewerId.trim();
      if (reviewNeedsSecond && !isUuid(normalizedSecondReviewerId)) {
        throw new Error("当前 M-VR 策略要求填写有效的第二复核员用户 ID");
      }
      if (normalizedSecondReviewerId === reviewerId) {
        throw new Error("第二复核员不能与第一复核员相同");
      }
      const lines = (order.lines ?? []).map((line) => ({
        line_no: line.line_no,
        product_code: line.product_code,
        reviewed_qty: line.picked_qty,
      }));
      if (lines.length === 0) throw new Error("复核订单没有可提交的明细");
      const reviewed = await reviewOutboundOrderMutation.mutateAsync({
        orderId: action.targetId,
        request: {
          reviewer_id: reviewerId,
          review_mode: "packing_station",
          second_reviewer_id: normalizedSecondReviewerId || null,
          lines,
        },
      });
      setOrders((value) => value.map((item) => item.id === reviewed.id ? reviewed : item));
      setSelectedId(reviewed.id);
      setLastEvent(`${reviewed.wms_order_no} 已复核`);
      return;
    }
    if (action.kind === "print") setLastEvent("打印任务已提交");
    if (action.kind === "ship") {
      const shipped = await shipOutboundOrderMutation.mutateAsync({ orderId: action.targetId, request: buildShipOutboundRequest(shipForm) });
      setOrders((value) => value.map((item) => item.id === shipped.id ? shipped : item));
      setSelectedId(shipped.id);
      setLastEvent(`${shipped.wms_order_no} 发货交接已完成`);
      return;
    }
    if (action.kind === "release-wave") updateWave(action.targetId, "inventory_locked", "波次已下发");
    if (action.kind === "approve-return") updateReturn(action.targetId, "approved", "采购退货审批已通过");
    if (action.kind === "reject-return") updateReturn(action.targetId, "cancelled", "采购退货审批已驳回");
    if (action.kind === "pick-return") updateReturn(action.targetId, "picking", "采购退货拣货已完成");
    if (action.kind === "review-return") updateReturn(action.targetId, "reviewed", "采购退货复核已完成");
    if (action.kind === "ship-return") updateReturn(action.targetId, "shipped", "采购退货出库交接已完成");
  }

  function updateOrder(id: string, status: string, message: string) {
    setOrders((value) => value.map((order) => order.id === id ? { ...order, status, updated_at: new Date().toISOString() } : order));
    setLastEvent(message);
  }

  function updateWave(id: string, status: string, message: string) {
    setWaves((value) => value.map((wave) => wave.id === id ? { ...wave, status, updated_at: new Date().toISOString() } : wave));
    setLastEvent(message);
  }

  function updateReturn(id: string, status: string, message: string) {
    setReturns((value) => value.map((item) => item.id === id ? { ...item, status } : item));
    setLastEvent(message);
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title={meta.title}
        subtitle={meta.subtitle}
      />
      {lastEvent && (
        <div
          className="fixed right-6 top-5 z-50 rounded-md border bg-background/95 px-3 py-2 text-sm text-muted-foreground shadow-sm"
          role="status"
        >
          {lastEvent}
        </div>
      )}

      {ordersQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {ordersQuery.error.message}
        </div>
      )}
      {wavesQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {wavesQuery.error.message}
        </div>
      )}
      {detailOpen && orderDetailQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {orderDetailQuery.error.message}
        </div>
      )}
      {detailOpen && waveDetailQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive" role="alert">
          {waveDetailQuery.error.message}
        </div>
      )}

      <QueryPanel
        fields={m4OutboundQueryFields}
        defaultVisibleFieldKeys={m4OutboundCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeM4OutboundQueryValue(next))}
        onQuery={() => {
          setAppliedQuery(normalizeM4OutboundQueryValue(draftQuery));
          setLastEvent(`${meta.title}已查询`);
        }}
        onReset={() => {
          const next = defaultM4OutboundQueryValue();
          setDraftQuery(next);
          setAppliedQuery(next);
          setSelectedId(null);
        }}
      />

      {mode === "waves" ? (
        <DataGrid
          storageKey="m4.outbound.waves"
          columns={waveColumns}
          data={filteredWaves}
          rowKey={(row) => row.id}
          selectedKey={selectedId ?? undefined}
          selectedRowKeys={selectedId ? [selectedId] : []}
          onSelectedRowKeysChange={(keys) => setSelectedId(keys.at(-1) ?? null)}
          onRowClick={(row) => setSelectedId(row.id)}
          emptyTitle="暂无波次"
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={(queryState) => {
            const next = normalizeM4OutboundQueryValue(queryValueFromUnknown(queryState));
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          onClearQueryState={() => {
            const next = defaultM4OutboundQueryValue();
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          selectable
        />
      ) : mode === "returns" ? (
        <DataGrid
          storageKey="m4.outbound.returns"
          columns={returnColumns}
          data={filteredReturns}
          rowKey={(row) => row.id}
          selectedKey={selectedId ?? undefined}
          selectedRowKeys={selectedId ? [selectedId] : []}
          onSelectedRowKeysChange={(keys) => setSelectedId(keys.at(-1) ?? null)}
          onRowClick={(row) => setSelectedId(row.id)}
          emptyTitle="暂无采购退货单"
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={(queryState) => {
            const next = normalizeM4OutboundQueryValue(queryValueFromUnknown(queryState));
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          onClearQueryState={() => {
            const next = defaultM4OutboundQueryValue();
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          selectable
        />
      ) : (
        <DataGrid
          storageKey={`m4.outbound.${mode}`}
          columns={orderColumns}
          data={filteredOrders}
          rowKey={(row) => row.id}
          selectedKey={selectedId ?? undefined}
          selectedRowKeys={selectedId ? [selectedId] : []}
          onSelectedRowKeysChange={(keys) => setSelectedId(keys.at(-1) ?? null)}
          onRowClick={(row) => setSelectedId(row.id)}
          emptyTitle="暂无出库单"
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={(queryState) => {
            const next = normalizeM4OutboundQueryValue(queryValueFromUnknown(queryState));
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          onClearQueryState={() => {
            const next = defaultM4OutboundQueryValue();
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          selectable
        />
      )}

      <M4OutboundActionDialog
        action={activeAction}
        target={resolveActionTarget(activeAction)}
        createForm={createForm}
        shipForm={shipForm}
        documentTypeOptions={documentTypeOptions}
        deliveryAddressOptions={deliveryAddressOptions}
        reviewOrder={activeAction?.kind === "review" ? reviewDetailQuery.data ?? null : null}
        reviewLoading={activeAction?.kind === "review" && reviewDetailQuery.isPending}
        reviewError={activeAction?.kind === "review" ? reviewDetailQuery.error?.message ?? null : null}
        reviewPolicy={reviewPolicy}
        reviewPolicyLoading={reviewPolicyLoading}
        secondReviewerId={secondReviewerId}
        note={note}
        actionError={actionError ?? (
          activeAction?.kind === "create-order"
            ? createOutboundOrderMutation.error?.message
              ?? documentTypeOptionsQuery.error?.message
              ?? deliveryAddressesQuery.error?.message
              ?? null
            : activeAction?.kind === "create-wave" ? createOutboundWaveMutation.error?.message ?? null
              : activeAction?.kind === "cancel-wave" ? cancelOutboundWaveMutation.error?.message ?? null
                : activeAction?.kind === "review" ? reviewOutboundOrderMutation.error?.message ?? null
                  : activeAction?.kind === "ship" ? shipOutboundOrderMutation.error?.message ?? null : null
        )}
        pending={
          (createOutboundOrderMutation.isPending && activeAction?.kind === "create-order")
          || (createOutboundWaveMutation.isPending && activeAction?.kind === "create-wave")
          || (cancelOutboundWaveMutation.isPending && activeAction?.kind === "cancel-wave")
          || (reviewOutboundOrderMutation.isPending && activeAction?.kind === "review")
          || (shipOutboundOrderMutation.isPending && activeAction?.kind === "ship")
          || (reviewDetailQuery.isPending && activeAction?.kind === "review")
          || (reviewPolicyLoading && activeAction?.kind === "review")
        }
        setCreateForm={setCreateForm}
        setShipForm={setShipForm}
        setNote={(value) => {
          setNote(value);
          if (actionError) setActionError(null);
        }}
        setSecondReviewerId={(value) => {
          setSecondReviewerId(value);
          if (actionError) setActionError(null);
        }}
        onClose={() => {
          setActiveAction(null);
          setActionError(null);
        }}
        onSubmit={submitAction}
      />
      <M4OutboundDetailDialog target={renderedDetailTarget} open={detailOpen} onOpenChange={setDetailOpen} />
    </section>
  );
}
