import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
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
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
  type StatusKey,
} from "@wms/ui";
import { CheckCircle2, ClipboardCheck, Truck } from "lucide-react";

import {
  M4OutboundDetailDialog,
  statusLabel,
  type DetailTarget,
  type OutboundOrder,
  type OutboundWave,
  type PurchaseReturnOrder,
} from "./M4OutboundDetailDialog";
import {
  ActionExtraFields,
  OrderNoSummary,
  ProductSummary,
  ReviewSummary,
  TextField,
  TwoLine,
  purchaseReturnApprovalSourceLabel,
  purchaseReturnDocumentTypeLabel,
} from "./M4OutboundPageParts";

export type M4OutboundMode = "orders" | "waves" | "review" | "returns";

interface M4OutboundPageProps {
  mode: M4OutboundMode;
  onBack: () => void;
}

type ActionKind =
  | "create-order"
  | "validate"
  | "void"
  | "create-wave"
  | "release-wave"
  | "cancel-wave"
  | "review"
  | "print"
  | "ship"
  | "create-return"
  | "approve-return"
  | "pick-return"
  | "review-return"
  | "ship-return";

interface ActionState {
  kind: ActionKind;
  targetId?: string;
}

const ownerId = "00000000-0000-0000-0000-000000000001";
const warehouseId = "00000000-0000-0000-0000-000000003001";
const customerId = "00000000-0000-0000-0000-000000004001";

// ponytail: local page state; replace with outbound list/detail hooks when the API exposes GET endpoints.
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

const m4OutboundStatusOptions = [
  { value: "draft", label: "草稿" },
  { value: "pending_validation", label: "待校验" },
  { value: "validation_exception", label: "校验异常" },
  { value: "confirmed", label: "已确认" },
  { value: "void_requested", label: "作废申请中" },
  { value: "in_wave", label: "已进波次" },
  { value: "inventory_locked", label: "库存锁定" },
  { value: "released", label: "已下发" },
  { value: "reviewed", label: "已复核" },
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
  const [createForm, setCreateForm] = React.useState({
    wmsOrderNo: "SO-M4-PC-NEW",
    erpOrderNo: "ERP-SO-NEW",
    orderType: "销售出库",
    customerName: "连锁门店 A",
    productCode: "P-M4-NEW",
    batchNo: "BATCH-OUT-202607",
    plannedQty: "12",
    requiredShipDate: "",
  });

  React.useEffect(() => {
    setSelectedId(null);
    const next = defaultM4OutboundQueryValue();
    setDraftQuery(next);
    setAppliedQuery(next);
    setLastEvent(null);
  }, [mode]);

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
    onClick: refreshOutbound,
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

  const orderColumns: DataGridColumn<OutboundOrder>[] = [
    { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 180, onDoubleClick: (row) => openOrderDetail(row.id), render: (row) => <OrderNoSummary order={row} /> },
    { key: "product", header: mode === "review" ? "复核 / 数量" : "商品 / 数量", minWidth: 220, render: (row) => mode === "review" ? <ReviewSummary order={row} /> : <ProductSummary order={row} /> },
    { key: "customer_id", header: mode === "review" ? "客户 / 配送" : "客户 / 门店", mono: true, minWidth: 170, render: (row) => mode === "review" ? <TwoLine top={`${shortId(row.customer_id)} / 门店A`} bottom="配送方 第三方快递" /> : `${shortId(row.customer_id)} / 门店A` },
    { key: "required_ship_at", header: mode === "review" ? "包裹 / 车牌" : "要求发货", minWidth: 150, render: (row) => mode === "review" ? <TwoLine top="包裹数量 1" bottom="车牌号 沪A-12345" /> : formatDate(row.required_ship_at) },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
  ];

  const waveColumns: DataGridColumn<OutboundWave>[] = [
    { key: "wave_no", header: "波次号", mono: true, minWidth: 180, onDoubleClick: (row) => openWaveDetail(row.id), render: (row) => <span className="text-primary">{row.wave_no}</span> },
    { key: "orders", header: "订单 / 明细", minWidth: 140, render: (row) => `${row.order_ids.length} 单 / ${waveLineCount(row, orders)} 行` },
    { key: "qty", header: "件数 / 温区", align: "right", minWidth: 130, render: (row) => `${waveQty(row, orders)} 件 / 常温` },
    { key: "route", header: "路径策略 / 容量", minWidth: 180, render: () => <TwoLine top="S 型最短路径" bottom="容量上限 100 单 / 10000 件" /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
  ];

  const returnColumns: DataGridColumn<PurchaseReturnOrder>[] = [
    { key: "return_no", header: "采购退货单 / 类型", mono: true, minWidth: 190, onDoubleClick: (row) => openReturnDetail(row.id), render: (row) => <TwoLine top={row.return_no} bottom={purchaseReturnDocumentTypeLabel(row.document_type)} /> },
    { key: "source_purchase_order_no", header: "原采购入库单", mono: true, minWidth: 180 },
    { key: "supplier_name", header: "供应商 / 原因", minWidth: 200, render: (row) => <TwoLine top={row.supplier_name} bottom={row.reason} /> },
    { key: "product", header: "商品 / 数量", minWidth: 160, render: (row) => <TwoLine top={row.product_code} bottom={`${row.qty} 件`} /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
  ];

  function refreshOutbound() {
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
    setDetailTarget({ kind: "wave", value: wave, orders: orders.filter((order) => wave.order_ids.includes(order.id)) });
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
    setActiveAction({ kind, targetId });
    setNote("");
  }

  function submitAction(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!activeAction) return;
    applyAction(activeAction);
    setActiveAction(null);
  }

  function applyAction(action: ActionState) {
    if (action.kind === "create-order") {
      const now = new Date().toISOString();
      const order = makeOrder(crypto.randomUUID(), createForm.wmsOrderNo, createForm.erpOrderNo, "pending_validation", toInteger(createForm.plannedQty), false, now);
      order.required_ship_at = createForm.requiredShipDate ? `${createForm.requiredShipDate}T09:00:00.000Z` : order.required_ship_at;
      order.lines[0].product_code = createForm.productCode;
      order.lines[0].batch_no = createForm.batchNo;
      setOrders((value) => [order, ...value]);
      setLastEvent(`${order.wms_order_no} 已创建`);
      return;
    }
    if (action.kind === "create-wave") {
      const orderIds = orders.filter((order) => order.status === "confirmed").map((order) => order.id).slice(0, 2);
      const wave = makeWave(`WAVE-M4-PC-${Date.now()}`, orderIds);
      setWaves((value) => [wave, ...value]);
      setOrders((value) => value.map((order) => orderIds.includes(order.id) ? { ...order, status: "in_wave" } : order));
      setLastEvent(`${wave.wave_no} 已创建`);
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
    if (action.kind === "review") updateOrder(action.targetId, "reviewed", "复核已完成");
    if (action.kind === "print") setLastEvent("打印任务已提交");
    if (action.kind === "ship") updateOrder(action.targetId, "shipped", "发货交接已完成");
    if (action.kind === "release-wave") updateWave(action.targetId, "inventory_locked", "波次已下发");
    if (action.kind === "cancel-wave") updateWave(action.targetId, "cancelled", "波次已取消");
    if (action.kind === "approve-return") updateReturn(action.targetId, "approved", "采购退货审批已通过");
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

      <ActionDialog
        action={activeAction}
        createForm={createForm}
        note={note}
        setCreateForm={setCreateForm}
        setNote={setNote}
        onClose={() => setActiveAction(null)}
        onSubmit={submitAction}
      />
      <M4OutboundDetailDialog target={detailTarget} open={detailOpen} onOpenChange={setDetailOpen} />
    </section>
  );
}

function outboundPrivateActions(
  mode: M4OutboundMode,
  selectedOrder: OutboundOrder | null,
  selectedWave: OutboundWave | null,
  selectedReturn: PurchaseReturnOrder | null,
  onAction: (kind: ActionKind, id: string) => void,
): DataGridToolbarAction[] {
  if (mode === "orders") {
    return [
      toolbarAction("validate", "校验", "重新校验", <CheckCircle2 className="size-4" aria-hidden />, selectedOrder?.id, onAction),
      toolbarAction("void", "作废", "作废申请", <ClipboardCheck className="size-4" aria-hidden />, selectedOrder?.id, onAction),
    ];
  }

  if (mode === "waves") {
    return [
      toolbarAction("release-wave", "下发", "下发波次", <CheckCircle2 className="size-4" aria-hidden />, selectedWave?.id, onAction),
      toolbarAction("cancel-wave", "取消", "取消波次", <ClipboardCheck className="size-4" aria-hidden />, selectedWave?.id, onAction),
    ];
  }

  if (mode === "review") {
    return [
      toolbarAction("review", "复核", "出库复核", <ClipboardCheck className="size-4" aria-hidden />, selectedOrder?.id, onAction),
      toolbarAction("ship", "交接", "发货交接", <Truck className="size-4" aria-hidden />, selectedOrder?.id, onAction),
    ];
  }

  return [
    toolbarAction("approve-return", "审批", "采购退货审批", <CheckCircle2 className="size-4" aria-hidden />, selectedReturn?.id, onAction),
    toolbarAction("pick-return", "拣货", "采购退货拣货", <Truck className="size-4" aria-hidden />, selectedReturn?.id, onAction),
    toolbarAction("review-return", "复核", "采购退货复核", <ClipboardCheck className="size-4" aria-hidden />, selectedReturn?.id, onAction),
    toolbarAction("ship-return", "出库", "采购退货出库交接", <CheckCircle2 className="size-4" aria-hidden />, selectedReturn?.id, onAction),
  ];
}

function toolbarAction(
  kind: ActionKind,
  label: string,
  description: string,
  icon: React.ReactNode,
  targetId: string | undefined,
  onAction: (kind: ActionKind, id: string) => void,
): DataGridToolbarAction {
  return {
    key: kind,
    label,
    description,
    icon,
    disabled: !targetId,
    onClick: () => {
      if (targetId) onAction(kind, targetId);
    },
  };
}

function ActionDialog({ action, createForm, note, setCreateForm, setNote, onClose, onSubmit }: {
  action: ActionState | null;
  createForm: { wmsOrderNo: string; erpOrderNo: string; orderType: string; customerName: string; productCode: string; batchNo: string; plannedQty: string; requiredShipDate: string };
  note: string;
  setCreateForm: React.Dispatch<React.SetStateAction<{ wmsOrderNo: string; erpOrderNo: string; orderType: string; customerName: string; productCode: string; batchNo: string; plannedQty: string; requiredShipDate: string }>>;
  setNote: (value: string) => void;
  onClose: () => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  if (!action) return null;
  const meta = actionMeta(action.kind);
  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <form className="grid gap-3 md:grid-cols-2" onSubmit={onSubmit}>
          <DialogHeader className="md:col-span-2">
            <DialogTitle>{meta.title}</DialogTitle>
            <DialogDescription>{meta.description}</DialogDescription>
          </DialogHeader>
          {action.kind === "create-order" ? (
            <>
              <TextField label="WMS 单号" value={createForm.wmsOrderNo} onChange={(wmsOrderNo) => setCreateForm((value) => ({ ...value, wmsOrderNo }))} />
              <TextField label="ERP 单号" value={createForm.erpOrderNo} onChange={(erpOrderNo) => setCreateForm((value) => ({ ...value, erpOrderNo }))} />
              <TextField label="订单类型" value={createForm.orderType} onChange={(orderType) => setCreateForm((value) => ({ ...value, orderType }))} />
              <TextField label="客户 / 门店" value={createForm.customerName} onChange={(customerName) => setCreateForm((value) => ({ ...value, customerName }))} />
              <TextField label="商品编码" value={createForm.productCode} onChange={(productCode) => setCreateForm((value) => ({ ...value, productCode }))} />
              <TextField label="批号" value={createForm.batchNo} onChange={(batchNo) => setCreateForm((value) => ({ ...value, batchNo }))} />
              <TextField label="计划数量" type="number" value={createForm.plannedQty} onChange={(plannedQty) => setCreateForm((value) => ({ ...value, plannedQty }))} />
              <TextField label="要求发货" type="date" value={createForm.requiredShipDate} onChange={(requiredShipDate) => setCreateForm((value) => ({ ...value, requiredShipDate }))} />
            </>
          ) : (
            <>
              <ActionExtraFields kind={action.kind} />
              <TextField className="md:col-span-2" label="备注" value={note} onChange={setNote} />
            </>
          )}
          <DialogFooter className="md:col-span-2">
            <DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose>
            <Button type="submit">{meta.submitLabel}</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function pageMeta(mode: M4OutboundMode) {
  const map = {
    orders: { title: "M4 出库订单管理", subtitle: "订单校验 · 双单号 · 作废申请", createAction: "create-order" as const, createLabel: "新建出库单" },
    waves: { title: "M4 波次规划", subtitle: "波次合并 · 库存锁定 · 路径策略", createAction: "create-wave" as const, createLabel: "新建波次" },
    review: { title: "M4 复核发货", subtitle: "包装站复核 · 打印 · 发货交接", createAction: null, createLabel: "" },
    returns: { title: "M4 采购退货出库", subtitle: "退供应商申请 · 审批 · 拣货复核 · 出库交接", createAction: "create-return" as const, createLabel: "新建采购退货单" },
  };
  return map[mode];
}

function actionMeta(kind: ActionKind) {
  const map: Record<ActionKind, { title: string; description: string; submitLabel: string }> = {
    "create-order": { title: "新建出库单", description: "手工创建 PC Web 测试出库单。", submitLabel: "创建出库单" },
    validate: { title: "重新校验", description: "重新执行库存和批号校验。", submitLabel: "确认校验" },
    void: { title: "作废申请", description: "提交未进波次订单的作废申请。", submitLabel: "提交申请" },
    "create-wave": { title: "新建波次", description: "把已确认订单合并为一个波次。", submitLabel: "创建波次" },
    "release-wave": { title: "下发波次", description: "下发波次并进入库存锁定。", submitLabel: "确认下发" },
    "cancel-wave": { title: "取消波次", description: "仅未开始拣选的波次可取消。", submitLabel: "确认取消" },
    review: { title: "复核", description: "包装站复核完成后提交。", submitLabel: "提交复核" },
    print: { title: "打印", description: "提交随货同行单或快递面单打印任务。", submitLabel: "提交打印" },
    ship: { title: "发货交接", description: "记录交接对象并确认发货。", submitLabel: "确认发货" },
    "create-return": { title: "新建采购退货单", description: "创建退供应商的出库申请。", submitLabel: "创建采购退货单" },
    "approve-return": { title: "采购退货审批", description: "审批退供应商出库申请。", submitLabel: "审批通过" },
    "pick-return": { title: "采购退货拣货", description: "记录退供应商出库拣货结果。", submitLabel: "确认拣货" },
    "review-return": { title: "采购退货复核", description: "复核退供应商商品和数量。", submitLabel: "提交复核" },
    "ship-return": { title: "采购退货出库交接", description: "确认退供应商出库交接。", submitLabel: "确认出库" },
  };
  return map[kind];
}

function statusOptions(mode: M4OutboundMode) {
  if (mode === "waves") return [["draft", "待下发"], ["released", "已下发"], ["inventory_locked", "库存锁定"], ["cancelled", "已取消"]];
  if (mode === "returns") return [["pending_approval", "待审批"], ["approved", "已审批"], ["picking", "拣货中"], ["reviewed", "已复核"], ["shipped", "已发货"]];
  return [["pending_validation", "待校验"], ["validation_exception", "校验异常"], ["confirmed", "已确认"], ["inventory_locked", "库存锁定"], ["reviewed", "已复核"], ["shipped", "已发货"]];
}

function statusKey(status: string): StatusKey {
  if (status.includes("exception") || status === "cancelled") return "unqualified";
  if (status === "completed" || status === "shipped" || status === "signed") return "completed";
  if (status === "inventory_locked" || status === "reviewed" || status === "released" || status === "pickup" || status === "inspecting" || status === "picking") return "in_progress";
  return "pending";
}

function defaultM4OutboundQueryValue(): QueryPanelValue {
  return {
    keyword: "",
    statusFilter: [],
    businessDate: { from: "", to: "" },
  };
}

function normalizeM4OutboundQueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    statusFilter: queryStringArray(value.statusFilter),
    businessDate: queryRange(value.businessDate),
  };
}

function filterOrders(orders: OutboundOrder[], query: QueryPanelValue, mode: M4OutboundMode) {
  const allowed = mode === "review" ? new Set(["inventory_locked", "reviewed", "shipped"]) : null;
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return orders.filter((order) => {
    const searchable = [order.wms_order_no, order.erp_order_no ?? "", order.customer_id, order.status, ...order.lines.flatMap((line) => [line.product_code, line.batch_no])].join(" ").toLowerCase();
    return (!allowed || allowed.has(order.status)) && matches(searchable, keyword) && matchesStatus(order.status, statuses) && dateInRange(order.required_ship_at, businessDate);
  });
}

function filterWaves(waves: OutboundWave[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return waves.filter((wave) => matches(`${wave.wave_no} ${wave.status}`.toLowerCase(), keyword) && matchesStatus(wave.status, statuses) && dateInRange(wave.created_at, businessDate));
}

function filterReturns(returns: PurchaseReturnOrder[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword);
  const statuses = new Set(queryStringArray(query.statusFilter));
  const businessDate = queryRange(query.businessDate);
  return returns.filter((item) => matches(
    [
      item.return_no,
      item.document_type,
      purchaseReturnDocumentTypeLabel(item.document_type),
      item.source_purchase_order_no,
      item.supplier_name,
      item.reason,
      item.product_code,
      item.approval_source,
      purchaseReturnApprovalSourceLabel(item.approval_source),
    ].join(" ").toLowerCase(),
    keyword,
  ) && matchesStatus(item.status, statuses) && dateInRange(item.created_at, businessDate));
}

function matches(searchable: string, keyword: string) {
  const normalized = keyword.trim().toLowerCase();
  return !normalized || searchable.includes(normalized);
}

function matchesStatus(status: string, statuses: Set<string>) {
  return statuses.size === 0 || statuses.has(status);
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryStringArray(value: QueryPanelValue[string]) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

function queryRange(value: QueryPanelValue[string]): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return { from: "", to: "" };
  return {
    from: typeof value.from === "string" ? value.from : "",
    to: typeof value.to === "string" ? value.to : "",
  };
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}

function dateInRange(value: string | null | undefined, range: QueryPanelRangeValue) {
  const date = value?.slice(0, 10) ?? "";
  return (!range.from || date >= range.from) && (!range.to || date <= range.to);
}

function makeOrder(id: string, wmsNo: string, erpNo: string, status: string, qty: number, shortPick: boolean, now = "2026-06-27T09:00:00.000Z"): OutboundOrder {
  return {
    id,
    owner_id: ownerId,
    customer_id: customerId,
    warehouse_id: warehouseId,
    wms_order_no: wmsNo,
    erp_order_no: erpNo,
    required_ship_at: "2026-06-28T09:00:00.000Z",
    status,
    short_pick: shortPick,
    created_at: now,
    updated_at: now,
    lines: [{ line_no: 1, product_code: "P-M4-001", batch_no: "BATCH-OUT-202606", planned_qty: qty, picked_qty: shortPick ? qty - 2 : qty, reviewed_qty: status === "reviewed" || status === "shipped" ? qty : 0, shipped_qty: status === "shipped" ? qty : 0, short_pick_qty: shortPick ? 2 : 0 }],
  };
}

function makeWave(waveNo: string, orderIds: string[]): OutboundWave {
  const now = new Date().toISOString();
  return { id: crypto.randomUUID(), owner_id: ownerId, wave_no: waveNo, status: "draft", order_ids: orderIds, created_at: now, updated_at: now };
}

function makeReturn(returnNo: string): PurchaseReturnOrder {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    return_no: returnNo,
    document_type: "purchase_return_outbound",
    source_purchase_order_no: "ASN-M2-PC-0001",
    supplier_name: "华东医药供应商",
    reason: "供应商召回",
    approval_source: "purchase_return_approval",
    status: "pending_approval",
    product_code: "P-M4-001",
    qty: 3,
    created_at: now,
    updated_at: now,
  };
}

function totalPlannedQty(order: OutboundOrder) {
  return order.lines.reduce((sum, line) => sum + line.planned_qty, 0);
}

function waveQty(wave: OutboundWave, orders: OutboundOrder[]) {
  return orders.filter((order) => wave.order_ids.includes(order.id)).reduce((sum, order) => sum + totalPlannedQty(order), 0);
}

function waveLineCount(wave: OutboundWave, orders: OutboundOrder[]) {
  return orders.filter((order) => wave.order_ids.includes(order.id)).reduce((sum, order) => sum + order.lines.length, 0);
}

function shortId(value: string) {
  return value.slice(0, 8);
}

function formatDate(value: string | null | undefined) {
  if (!value) return "-";
  return value.slice(0, 10);
}

function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}
