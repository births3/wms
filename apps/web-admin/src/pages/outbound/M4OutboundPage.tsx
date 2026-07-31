import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
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
} from "./M4OutboundDetailDialog";
import {
  M4OutboundActionDialog,
  outboundPrivateActions,
  type ActionKind,
  type ActionState,
  type ActionTargetContext,
  type OutboundCreateForm,
  type PurchaseReturnCreateForm,
} from "./M4OutboundActionDialog";
import {
  useApprovePurchaseReturnMutation,
  useCancelOutboundWaveMutation,
  useCreateOutboundOrderMutation,
  useCreateOutboundWaveMutation,
  useCreatePurchaseReturnMutation,
  useOutboundOrderQuery,
  useOutboundOrdersQuery,
  useOutboundReviewQuery,
  useOutboundWaveQuery,
  useOutboundWavesQuery,
  usePickPurchaseReturnMutation,
  usePurchaseReturnsQuery,
  useRejectPurchaseReturnMutation,
  useReleaseOutboundWaveMutation,
  useRevalidateOutboundOrderMutation,
  useReviewOutboundOrderMutation,
  useReviewPurchaseReturnMutation,
  useShipOutboundOrderMutation,
  useShipPurchaseReturnMutation,
  useVoidRequestOutboundOrderMutation,
  OUTBOUND_LIST_LIMIT,
  type CreateOutboundWaveRequest,
} from "@/features/outbound/outbound-queries";
import { useCurrentUserQuery } from "@/features/auth/auth-queries";
import { useCustomerAddressesQuery, useMasterDataRowsQuery, useSystemDictionaryItemOptionsQuery } from "@/features/master-data/master-data-queries";
import {
  useDualPersonPolicyQueries,
  type DualPersonPolicy,
} from "@/features/validation-rules/dual-person-policy-queries";
import {
  OutboundPageErrors,
} from "./M4OutboundPageParts";
import {
  defaultM4OutboundQueryValue,
  filterOrders,
  filterReturns,
  filterWaves,
  normalizeM4OutboundQueryValue,
  queryValueFromUnknown,
  pageMeta,
  statusLabel,
  statusOptions,
  type DetailTarget,
  type M4OutboundMode,
  type OutboundOrder,
  type OutboundWave,
  type PurchaseReturnOrder,
} from "./m4-outbound-page-model";
import {
  outboundOrderColumns,
  outboundWaveColumns,
  purchaseReturnColumns,
} from "./M4OutboundGridColumns";
import {
  H9BusinessPrintDialog,
  type H9BusinessPrintTarget,
} from "../print-template/H9BusinessPrintDialog";
import {
  emptyPurchaseReturnForm,
  outboundOrderRequest,
  purchaseReturnRequest,
} from "./m4-outbound-action-requests";
import {
  buildShipOutboundRequest,
  defaultOutboundShipForm,
  type OutboundShipForm,
} from "./m4-outbound-page-helpers";
import { outboundPrintTarget } from "./m4-outbound-print";
import { useDialogState } from "@/lib/use-dialog-state";
import { usePageQueryState } from "@/lib/use-page-query-state";
import { useTransientEvent } from "@/lib/use-transient-event";
import { isUuid } from "@/lib/uuid";

export type { M4OutboundMode } from "./m4-outbound-page-model";

interface M4OutboundPageProps {
  mode: M4OutboundMode;
  onBack: () => void;
}

const m4OutboundQueryFields: QueryPanelField[] = [
  { key: "keyword", label: "关键字", type: "text" },
  { key: "statusFilter", label: "状态", type: "select" },
];
const m4OutboundCoreQueryFieldKeys = ["keyword", "statusFilter"];

export function M4OutboundPage({ mode }: M4OutboundPageProps) {
  // 订单 / 波次 / 采购退货均以空列表起步、由真实查询填充：接口加载中或失败时不得把演示单据当真数据展示。
  const [orders, setOrders] = React.useState<OutboundOrder[]>([]);
  const [waves, setWaves] = React.useState<OutboundWave[]>([]);
  const [returns, setReturns] = React.useState<PurchaseReturnOrder[]>([]);
  const { draftQuery, setDraftQuery, appliedQuery, applyQuery, resetQuery } =
    usePageQueryState<QueryPanelValue>(defaultM4OutboundQueryValue, normalizeM4OutboundQueryValue);
  const normalizedQuery = normalizeM4OutboundQueryValue(appliedQuery);
  const listQuery = React.useMemo(
    () => ({
      q: typeof normalizedQuery.keyword === "string" && normalizedQuery.keyword.trim() ? normalizedQuery.keyword.trim() : undefined,
      status: typeof normalizedQuery.statusFilter === "string" && normalizedQuery.statusFilter.trim() ? normalizedQuery.statusFilter.trim() : undefined,
      limit: OUTBOUND_LIST_LIMIT,
    }),
    [normalizedQuery.keyword, normalizedQuery.statusFilter],
  );
  const ordersListQuery = mode === "orders" || mode === "review" ? listQuery : { limit: OUTBOUND_LIST_LIMIT };
  const wavesListQuery = mode === "waves" ? listQuery : { limit: OUTBOUND_LIST_LIMIT };
  const returnsListQuery = mode === "returns" ? listQuery : { limit: OUTBOUND_LIST_LIMIT };
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const {
    open: detailOpen,
    target: detailTarget,
    openWith: openDetailWith,
    setOpen: setDetailOpen,
  } = useDialogState<DetailTarget>();
  const [activeAction, setActiveAction] = React.useState<ActionState | null>(null);
  const [printTarget, setPrintTarget] = React.useState<H9BusinessPrintTarget | null>(null);
  const { lastEvent, setLastEvent } = useTransientEvent();
  const [note, setNote] = React.useState("");
  const [actionError, setActionError] = React.useState<string | null>(null);
  const [secondReviewerId, setSecondReviewerId] = React.useState("");
  const [createForm, setCreateForm] = React.useState<OutboundCreateForm>({
    wmsOrderNo: "",
    erpOrderNo: "",
    documentType: "sales_outbound",
    warehouseId: "",
    customerId: "",
    deliveryAddressId: "",
    productCode: "",
    batchNo: "",
    plannedQty: "",
    requiredShipDate: "",
  });
  const [purchaseReturnForm, setPurchaseReturnForm] =
    React.useState<PurchaseReturnCreateForm>(emptyPurchaseReturnForm);
  const [shipForm, setShipForm] = React.useState<OutboundShipForm>(defaultOutboundShipForm);
  const ordersQuery = useOutboundOrdersQuery(ordersListQuery, mode === "orders" || mode === "waves" || mode === "review");
  const wavesQuery = useOutboundWavesQuery(wavesListQuery, mode === "waves");
  const returnsQuery = usePurchaseReturnsQuery(returnsListQuery, mode === "returns");
  const createOutboundOrderMutation = useCreateOutboundOrderMutation();
  const createOutboundWaveMutation = useCreateOutboundWaveMutation();
  const releaseOutboundWaveMutation = useReleaseOutboundWaveMutation();
  const cancelOutboundWaveMutation = useCancelOutboundWaveMutation();
  const reviewOutboundOrderMutation = useReviewOutboundOrderMutation();
  const revalidateOutboundOrderMutation = useRevalidateOutboundOrderMutation();
  const voidRequestOutboundOrderMutation = useVoidRequestOutboundOrderMutation();
  const shipOutboundOrderMutation = useShipOutboundOrderMutation();
  const createPurchaseReturnMutation = useCreatePurchaseReturnMutation();
  const approvePurchaseReturnMutation = useApprovePurchaseReturnMutation();
  const rejectPurchaseReturnMutation = useRejectPurchaseReturnMutation();
  const pickPurchaseReturnMutation = usePickPurchaseReturnMutation();
  const reviewPurchaseReturnMutation = useReviewPurchaseReturnMutation();
  const shipPurchaseReturnMutation = useShipPurchaseReturnMutation();
  const currentUserQuery = useCurrentUserQuery(true);
  const canPrint = currentUserQuery.data?.permissions.includes("h9.print_template.print") ?? false;
  const orderDetailQuery = useOutboundOrderQuery(detailTarget?.kind === "order" ? detailTarget.value.id : null);
  const waveDetailQuery = useOutboundWaveQuery(detailTarget?.kind === "wave" ? detailTarget.value.id : null);
  const reviewDetailQuery = useOutboundReviewQuery(activeAction?.kind === "review" ? activeAction.targetId ?? null : null);
  const reviewProductsQuery = useMasterDataRowsQuery("m1-products", activeAction?.kind === "review");
  const createWarehousesQuery = useMasterDataRowsQuery(
    "m1-warehouses",
    mode === "orders" || mode === "returns",
  );
  const createPartnersQuery = useMasterDataRowsQuery("m1-business-partners", mode === "orders");
  const createAddressesQuery = useCustomerAddressesQuery(createForm.customerId || null);
  const createWarehouses = React.useMemo(() => createWarehousesQuery.data ?? [], [createWarehousesQuery.data]);
  const createCustomers = React.useMemo(
    () => (createPartnersQuery.data ?? []).filter((item) => item.partnerKind === "customer"),
    [createPartnersQuery.data],
  );
  const createAddresses = React.useMemo(() => createAddressesQuery.data ?? [], [createAddressesQuery.data]);
  React.useEffect(() => {
    if (mode !== "orders") return;
    setCreateForm((current) => ({
      ...current,
      warehouseId: createWarehouses.some((item) => item.id === current.warehouseId) ? current.warehouseId : createWarehouses[0]?.id ?? "",
      customerId: createCustomers.some((item) => item.id === current.customerId) ? current.customerId : createCustomers[0]?.id ?? "",
    }));
  }, [createCustomers, createWarehouses, mode]);
  React.useEffect(() => {
    if (mode !== "returns") return;
    setPurchaseReturnForm((current) => ({
      ...current,
      warehouseId: createWarehouses.some((item) => item.id === current.warehouseId)
        ? current.warehouseId
        : createWarehouses[0]?.id ?? "",
    }));
  }, [createWarehouses, mode]);
  React.useEffect(() => {
    setCreateForm((current) => ({
      ...current,
      deliveryAddressId: createAddresses.some((item) => item.id === current.deliveryAddressId) ? current.deliveryAddressId : createAddresses[0]?.id ?? "",
    }));
  }, [createAddresses]);
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
    resetQuery();
    setLastEvent(null);
  }, [mode, resetQuery, setLastEvent]);

  React.useEffect(() => {
    if (!ordersQuery.isPending && !ordersQuery.error && ordersQuery.data) setOrders(ordersQuery.data);
  }, [ordersQuery.data, ordersQuery.error, ordersQuery.isPending]);

  React.useEffect(() => {
    if (!wavesQuery.isPending && !wavesQuery.error && wavesQuery.data) setWaves(wavesQuery.data);
  }, [wavesQuery.data, wavesQuery.error, wavesQuery.isPending]);

  React.useEffect(() => {
    if (!returnsQuery.isPending && !returnsQuery.error && returnsQuery.data) setReturns(returnsQuery.data);
  }, [returnsQuery.data, returnsQuery.error, returnsQuery.isPending]);

  const meta = pageMeta(mode);
  const filteredOrders = filterOrders(ordersQuery.data ?? [], normalizedQuery, mode);
  const filteredWaves = filterWaves(wavesQuery.data ?? [], normalizedQuery);
  const filteredReturns = filterReturns(returnsQuery.data ?? [], normalizedQuery);
  const selectedOrder = filteredOrders.find((item) => item.id === selectedId) ?? null;
  const selectedWave = filteredWaves.find((item) => item.id === selectedId) ?? null;
  const selectedReturn = filteredReturns.find((item) => item.id === selectedId) ?? null;
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(
      m4OutboundQueryFields.map((field) => field.key === "statusFilter"
        ? { ...field, options: statusOptions(mode).map(([value, label]) => ({ value, label })) }
        : field),
      appliedQuery,
    ),
    [appliedQuery, mode],
  );
  const listWindowLimited = (mode === "orders" || mode === "review")
    ? (ordersQuery.data?.length ?? 0) >= OUTBOUND_LIST_LIMIT
    : mode === "waves"
      ? (wavesQuery.data?.length ?? 0) >= OUTBOUND_LIST_LIMIT
      : (returnsQuery.data?.length ?? 0) >= OUTBOUND_LIST_LIMIT;
  const createActionKind = meta.createAction;
  const gridRefreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: `刷新${meta.title}列表`,
    disabled: ordersQuery.isFetching || wavesQuery.isFetching || returnsQuery.isFetching,
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
  const gridPrintAction: DataGridPrintAction | undefined =
    canPrint && (mode === "orders" || mode === "review")
      ? {
          label: "打印",
          description: "打印随货同行单",
          disabled: ({ selectedRowKeys }) => selectedRowKeys.length !== 1,
          onClick: ({ selectedRowKeys }) => {
            const order = orders.find((item) => item.id === selectedRowKeys[0]);
            if (order) setPrintTarget(outboundPrintTarget(order));
          },
        }
      : undefined;
  const gridExportAction: DataGridExportAction = {
    label: "导出",
    description: `导出${meta.title}`,
  };
  const gridToolbarActions = outboundPrivateActions(mode, selectedOrder, selectedWave, selectedReturn, openAction);

  const orderColumns = outboundOrderColumns(mode, openOrderDetail);
  const waveColumns = outboundWaveColumns(mode, orders, openWaveDetail);
  const returnColumns = purchaseReturnColumns(mode, openReturnDetail);

  function applyGridQueryState(queryState: unknown) {
    applyQuery(queryValueFromUnknown(queryState));
    setSelectedId(null);
  }

  function clearGridQueryState() {
    resetQuery();
    setSelectedId(null);
  }

  async function refreshOutbound() {
    if (mode === "orders" || mode === "review") await ordersQuery.refetch();
    if (mode === "waves") await Promise.all([ordersQuery.refetch(), wavesQuery.refetch()]);
    if (mode === "returns") await returnsQuery.refetch();
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
    openDetailWith({ kind: "order", value: order });
  }

  function openWaveDetail(id: string) {
    const wave = waves.find((item) => item.id === id);
    if (!wave) return;
    setSelectedId(id);
    openDetailWith({ kind: "wave", value: wave, orders: orders.filter((order) => (wave.order_ids ?? []).includes(order.id)) });
  }

  function openReturnDetail(id: string) {
    const returnOrder = returns.find((item) => item.id === id);
    if (!returnOrder) return;
    setSelectedId(id);
    openDetailWith({ kind: "return", value: returnOrder });
  }

  function openAction(kind: ActionKind, targetId?: string) {
    if (kind === "create-order") createOutboundOrderMutation.reset();
    if (kind === "create-wave") createOutboundWaveMutation.reset();
    if (kind === "release-wave") releaseOutboundWaveMutation.reset();
    if (kind === "cancel-wave") cancelOutboundWaveMutation.reset();
    if (kind === "review") reviewOutboundOrderMutation.reset();
    if (kind === "validate") revalidateOutboundOrderMutation.reset();
    if (kind === "void") voidRequestOutboundOrderMutation.reset();
    if (kind === "ship") shipOutboundOrderMutation.reset();
    if (kind === "create-return") createPurchaseReturnMutation.reset();
    if (kind === "approve-return") approvePurchaseReturnMutation.reset();
    if (kind === "reject-return") rejectPurchaseReturnMutation.reset();
    if (kind === "pick-return") pickPurchaseReturnMutation.reset();
    if (kind === "review-return") reviewPurchaseReturnMutation.reset();
    if (kind === "ship-return") shipPurchaseReturnMutation.reset();
    setActiveAction({ kind, targetId });
    setNote("");
    setSecondReviewerId("");
    setActionError(null);
    if (kind === "create-return") {
      setPurchaseReturnForm((current) => ({
        ...emptyPurchaseReturnForm,
        warehouseId: current.warehouseId,
      }));
    }
    if (kind === "ship") setShipForm(defaultOutboundShipForm());
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
      const request = outboundOrderRequest(createForm);
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
      const request = purchaseReturnRequest(purchaseReturnForm);
      const item = await createPurchaseReturnMutation.mutateAsync(request);
      setReturns((value) => [item, ...value]);
      setLastEvent(`${item.return_no} 已创建`);
      return;
    }
    if (!action.targetId) return;
    if (action.kind === "validate") {
      const revalidated = await revalidateOutboundOrderMutation.mutateAsync(action.targetId);
      setOrders((value) => value.map((item) => item.id === revalidated.id ? revalidated : item));
      setLastEvent(revalidated.status === "validation_exception" ? "校验未通过" : "重新校验已完成");
      return;
    }
    if (action.kind === "void") {
      const voided = await voidRequestOutboundOrderMutation.mutateAsync(action.targetId);
      setOrders((value) => value.map((item) => item.id === voided.id ? voided : item));
      setLastEvent("作废申请已提交");
      return;
    }
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
    if (action.kind === "ship") {
      const shipped = await shipOutboundOrderMutation.mutateAsync({
        orderId: action.targetId,
        request: buildShipOutboundRequest(shipForm),
      });
      setOrders((value) => value.map((item) => item.id === shipped.id ? shipped : item));
      setLastEvent("发货交接已完成");
      return;
    }
    if (action.kind === "release-wave") {
      const released = await releaseOutboundWaveMutation.mutateAsync(action.targetId);
      setWaves((value) => value.map((wave) => wave.id === released.id ? released : wave));
      const releasedOrderIds = released.order_ids ?? [];
      setOrders((value) => value.map((order) => releasedOrderIds.includes(order.id) ? { ...order, status: "in_wave" } : order));
      setLastEvent("波次已下发");
      return;
    }
    if (action.kind === "approve-return") {
      applyReturnResult(await approvePurchaseReturnMutation.mutateAsync(action.targetId), "采购退货审批已通过");
      return;
    }
    if (action.kind === "reject-return") {
      applyReturnResult(
        await rejectPurchaseReturnMutation.mutateAsync({ returnId: action.targetId, request: { reason: note.trim() } }),
        "采购退货审批已驳回",
      );
      return;
    }
    if (action.kind === "pick-return") {
      applyReturnResult(await pickPurchaseReturnMutation.mutateAsync(action.targetId), "采购退货拣货已完成");
      return;
    }
    if (action.kind === "review-return") {
      applyReturnResult(await reviewPurchaseReturnMutation.mutateAsync(action.targetId), "采购退货复核已完成");
      return;
    }
    if (action.kind === "ship-return") {
      applyReturnResult(await shipPurchaseReturnMutation.mutateAsync(action.targetId), "采购退货出库交接已完成");
    }
  }

  function applyReturnResult(item: PurchaseReturnOrder, message: string) {
    setReturns((value) => value.map((current) => current.id === item.id ? item : current));
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

      <OutboundPageErrors messages={[
        ordersQuery.error?.message,
        wavesQuery.error?.message,
        returnsQuery.error?.message,
        detailOpen ? orderDetailQuery.error?.message : null,
        detailOpen ? waveDetailQuery.error?.message : null,
      ]} />

      <QueryPanel
        fields={m4OutboundQueryFields}
        fieldOptions={{
          statusFilter: statusOptions(mode).map(([value, label]) => ({ value, label })),
        }}
        defaultVisibleFieldKeys={m4OutboundCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeM4OutboundQueryValue(next))}
        onQuery={() => {
          applyQuery(draftQuery);
          setLastEvent(`${meta.title}已查询`);
        }}
        onReset={() => {
          resetQuery();
          setSelectedId(null);
        }}
      />

      {listWindowLimited && (
        <div className="rounded-md border border-amber-300 bg-amber-50 px-3 py-2 text-sm text-amber-900" role="status">
          当前窗口可能未完整，请收窄条件
        </div>
      )}

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
          caption={wavesQuery.isPending ? "加载波次..." : undefined}
          emptyTitle={wavesQuery.isError ? "读取波次失败" : "暂无波次"}
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={applyGridQueryState}
          onClearQueryState={clearGridQueryState}
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
          caption={returnsQuery.isPending ? "加载采购退货单..." : undefined}
          emptyTitle={returnsQuery.isError ? "读取采购退货单失败" : "暂无采购退货单"}
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={applyGridQueryState}
          onClearQueryState={clearGridQueryState}
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
          caption={ordersQuery.isPending ? "加载出库单..." : undefined}
          emptyTitle={ordersQuery.isError ? "读取出库单失败" : "暂无出库单"}
          exportFileBaseName={meta.title}
          refreshAction={gridRefreshAction}
          createAction={gridCreateAction}
          detailAction={gridDetailAction}
          printAction={gridPrintAction}
          exportAction={gridExportAction}
          toolbarActions={gridToolbarActions}
          queryState={appliedQuery}
          querySummaryItems={querySummaryItems}
          onApplyQueryState={applyGridQueryState}
          onClearQueryState={clearGridQueryState}
          selectable
        />
      )}

      <M4OutboundActionDialog
        action={activeAction}
        target={resolveActionTarget(activeAction)}
        createForm={createForm}
        purchaseReturnForm={purchaseReturnForm}
        shipForm={shipForm}
        documentTypeOptions={documentTypeOptions}
        warehouseOptions={createWarehouses.map((item) => ({ value: item.id, label: `${item.code} ${item.name}` }))}
        customerOptions={createCustomers.map((item) => ({ value: item.id, label: `${item.code} ${item.name}` }))}
        addressOptions={createAddresses.map((item) => ({
          value: item.id,
          label: `${item.province}${item.city}${item.district}${item.detail_address}`,
        }))}
        reviewOrder={activeAction?.kind === "review" ? reviewDetailQuery.data ?? null : null}
        reviewLoading={activeAction?.kind === "review" && reviewDetailQuery.isPending}
        reviewError={activeAction?.kind === "review" ? reviewDetailQuery.error?.message ?? null : null}
        reviewPolicy={reviewPolicy}
        reviewPolicyLoading={reviewPolicyLoading}
        secondReviewerId={secondReviewerId}
        note={note}
        actionError={actionError ?? (
          activeAction?.kind === "create-order"
            ? createOutboundOrderMutation.error?.message ?? documentTypeOptionsQuery.error?.message ?? null
            : activeAction?.kind === "create-wave" ? createOutboundWaveMutation.error?.message ?? null
              : activeAction?.kind === "release-wave" ? releaseOutboundWaveMutation.error?.message ?? null
                : activeAction?.kind === "cancel-wave" ? cancelOutboundWaveMutation.error?.message ?? null
                  : activeAction?.kind === "review" ? reviewOutboundOrderMutation.error?.message ?? null
                    : activeAction?.kind === "validate" ? revalidateOutboundOrderMutation.error?.message ?? null
                      : activeAction?.kind === "void" ? voidRequestOutboundOrderMutation.error?.message ?? null
                        : activeAction?.kind === "ship" ? shipOutboundOrderMutation.error?.message ?? null
                          : activeAction?.kind === "create-return" ? createPurchaseReturnMutation.error?.message ?? null
                            : activeAction?.kind === "approve-return" ? approvePurchaseReturnMutation.error?.message ?? null
                              : activeAction?.kind === "reject-return" ? rejectPurchaseReturnMutation.error?.message ?? null
                                : activeAction?.kind === "pick-return" ? pickPurchaseReturnMutation.error?.message ?? null
                                  : activeAction?.kind === "review-return" ? reviewPurchaseReturnMutation.error?.message ?? null
                                    : activeAction?.kind === "ship-return" ? shipPurchaseReturnMutation.error?.message ?? null : null
        )}
        pending={
          (createOutboundOrderMutation.isPending && activeAction?.kind === "create-order")
          || (createOutboundWaveMutation.isPending && activeAction?.kind === "create-wave")
          || (releaseOutboundWaveMutation.isPending && activeAction?.kind === "release-wave")
          || (cancelOutboundWaveMutation.isPending && activeAction?.kind === "cancel-wave")
          || (reviewOutboundOrderMutation.isPending && activeAction?.kind === "review")
          || (revalidateOutboundOrderMutation.isPending && activeAction?.kind === "validate")
          || (voidRequestOutboundOrderMutation.isPending && activeAction?.kind === "void")
          || (shipOutboundOrderMutation.isPending && activeAction?.kind === "ship")
          || (createPurchaseReturnMutation.isPending && activeAction?.kind === "create-return")
          || (approvePurchaseReturnMutation.isPending && activeAction?.kind === "approve-return")
          || (rejectPurchaseReturnMutation.isPending && activeAction?.kind === "reject-return")
          || (pickPurchaseReturnMutation.isPending && activeAction?.kind === "pick-return")
          || (reviewPurchaseReturnMutation.isPending && activeAction?.kind === "review-return")
          || (shipPurchaseReturnMutation.isPending && activeAction?.kind === "ship-return")
          || (reviewDetailQuery.isPending && activeAction?.kind === "review")
          || (reviewPolicyLoading && activeAction?.kind === "review")
        }
        setCreateForm={setCreateForm}
        setPurchaseReturnForm={setPurchaseReturnForm}
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
      <H9BusinessPrintDialog
        open={Boolean(printTarget)}
        target={printTarget}
        onOpenChange={(next) => {
          if (!next) setPrintTarget(null);
        }}
        onPrinted={(target) => setLastEvent(`${target.description}已登记打印结果`)}
      />
    </section>
  );
}

function strictestDualPersonPolicy(policies: DualPersonPolicy[]): DualPersonPolicy {
  if (policies.includes("dual_scan_with_approval")) return "dual_scan_with_approval";
  if (policies.includes("dual_scan")) return "dual_scan";
  return "single";
}
