import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  StatusBadge,
  type DataGridColumn,
  type StatusKey,
} from "@wms/ui";
import { ArrowLeft, CheckCircle2, ClipboardCheck, Eye, Plus, Printer, RefreshCw, Truck } from "lucide-react";

import {
  M4OutboundDetailDialog,
  statusLabel,
  type DetailTarget,
  type OutboundOrder,
  type OutboundWave,
  type PurchaseReturnOrder,
} from "./M4OutboundDetailDialog";

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

export function M4OutboundPage({ mode, onBack }: M4OutboundPageProps) {
  const [orders, setOrders] = React.useState<OutboundOrder[]>(seedOrders);
  const [waves, setWaves] = React.useState<OutboundWave[]>(seedWaves);
  const [returns, setReturns] = React.useState<PurchaseReturnOrder[]>(seedReturns);
  const [keyword, setKeyword] = React.useState("");
  const [statusFilter, setStatusFilter] = React.useState("all");
  const [dateFilter, setDateFilter] = React.useState("");
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
    setStatusFilter("all");
    setKeyword("");
    setDateFilter("");
    setLastEvent(null);
  }, [mode]);

  React.useEffect(() => {
    if (!lastEvent) return;
    const timer = window.setTimeout(() => setLastEvent(null), 3000);
    return () => window.clearTimeout(timer);
  }, [lastEvent]);

  const meta = pageMeta(mode);

  const orderColumns: DataGridColumn<OutboundOrder>[] = [
    { key: "wms_order_no", header: "单号 / 类型", mono: true, minWidth: 180, render: (row) => <OrderNoSummary order={row} /> },
    { key: "product", header: mode === "review" ? "复核 / 数量" : "商品 / 数量", minWidth: 220, render: (row) => mode === "review" ? <ReviewSummary order={row} /> : <ProductSummary order={row} /> },
    { key: "customer_id", header: mode === "review" ? "客户 / 配送" : "客户 / 门店", mono: true, minWidth: 170, render: (row) => mode === "review" ? <TwoLine top={`${shortId(row.customer_id)} / 门店A`} bottom="配送方 第三方快递" /> : `${shortId(row.customer_id)} / 门店A` },
    { key: "required_ship_at", header: mode === "review" ? "包裹 / 车牌" : "要求发货", minWidth: 150, render: (row) => mode === "review" ? <TwoLine top="包裹数量 1" bottom="车牌号 沪A-12345" /> : formatDate(row.required_ship_at) },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
    { key: "actions", header: "操作", align: "right", minWidth: 280, filter: false, sortable: false, copyable: false, hideable: false, render: (row) => <OrderActions row={row} mode={mode} onDetail={openOrderDetail} onAction={openAction} /> },
  ];

  const waveColumns: DataGridColumn<OutboundWave>[] = [
    { key: "wave_no", header: "波次号", mono: true, minWidth: 180, render: (row) => <span className="text-primary">{row.wave_no}</span> },
    { key: "orders", header: "订单 / 明细", minWidth: 140, render: (row) => `${row.order_ids.length} 单 / ${waveLineCount(row, orders)} 行` },
    { key: "qty", header: "件数 / 温区", align: "right", minWidth: 130, render: (row) => `${waveQty(row, orders)} 件 / 常温` },
    { key: "route", header: "路径策略 / 容量", minWidth: 180, render: () => <TwoLine top="S 型最短路径" bottom="容量上限 100 单 / 10000 件" /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
    { key: "actions", header: "操作", align: "right", minWidth: 220, filter: false, sortable: false, copyable: false, hideable: false, render: (row) => <WaveActions row={row} onDetail={openWaveDetail} onAction={openAction} /> },
  ];

  const returnColumns: DataGridColumn<PurchaseReturnOrder>[] = [
    { key: "return_no", header: "采购退货单 / 类型", mono: true, minWidth: 190, render: (row) => <TwoLine top={row.return_no} bottom={purchaseReturnDocumentTypeLabel(row.document_type)} /> },
    { key: "source_purchase_order_no", header: "原采购入库单", mono: true, minWidth: 180 },
    { key: "supplier_name", header: "供应商 / 原因", minWidth: 200, render: (row) => <TwoLine top={row.supplier_name} bottom={row.reason} /> },
    { key: "product", header: "商品 / 数量", minWidth: 160, render: (row) => <TwoLine top={row.product_code} bottom={`${row.qty} 件`} /> },
    { key: "created_at", header: "创建时间", minWidth: 150, filter: { type: "dateRange" }, render: (row) => formatDate(row.created_at) },
    { key: "status", header: "状态", minWidth: 130, filter: { type: "multiSelect", options: statusOptions(mode).map(([value, label]) => ({ value, label })) }, render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" /> },
    { key: "actions", header: "操作", align: "right", minWidth: 360, filter: false, sortable: false, copyable: false, hideable: false, render: (row) => <ReturnActions row={row} onDetail={openReturnDetail} onAction={openAction} /> },
  ];

  const filteredOrders = filterOrders(orders, keyword, statusFilter, dateFilter, mode);
  const filteredWaves = filterWaves(waves, keyword, statusFilter);
  const filteredReturns = filterReturns(returns, keyword, statusFilter);

  function refreshOutbound() {
    setLastEvent(`${meta.title}已刷新`);
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
    <section className="mx-auto flex w-full max-w-[1400px] flex-col gap-5 px-6 py-8">
      <PageHeader
        title={meta.title}
        subtitle={meta.subtitle}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button type="button" variant="outline" onClick={refreshOutbound}><RefreshCw className="size-4" aria-hidden />刷新</Button>
            <Button type="button" variant="outline" onClick={() => globalThis.print()}><Printer className="size-4" aria-hidden />打印</Button>
            {meta.createAction && <Button type="button" onClick={() => openAction(meta.createAction!)}><Plus className="size-4" aria-hidden />{meta.createLabel}</Button>}
            <Button type="button" variant="outline" onClick={onBack}><ArrowLeft className="size-4" aria-hidden />返回工作台</Button>
          </div>
        }
      />
      {lastEvent && (
        <div
          className="fixed right-6 top-5 z-50 rounded-md border bg-background/95 px-3 py-2 text-sm text-muted-foreground shadow-sm"
          role="status"
        >
          {lastEvent}
        </div>
      )}

      <div className="grid gap-3 md:grid-cols-4">
        {meta.metrics.map((metric) => <Metric key={metric.label} label={metric.label} value={metric.value({ orders, waves, returns })} tone={metric.tone} />)}
      </div>

      <FilterBar
        keyword={keyword}
        statusFilter={statusFilter}
        dateFilter={dateFilter}
        mode={mode}
        onKeywordChange={setKeyword}
        onStatusFilterChange={setStatusFilter}
        onDateFilterChange={setDateFilter}
        onReset={() => {
          setKeyword("");
          setStatusFilter("all");
          setDateFilter("");
        }}
      />

      {mode === "waves" ? (
        <DataGrid storageKey="m4.outbound.waves" columns={waveColumns} data={filteredWaves} rowKey={(row) => row.id} selectedKey={selectedId ?? undefined} onRowClick={(row) => setSelectedId(row.id)} emptyTitle="暂无波次" />
      ) : mode === "returns" ? (
        <DataGrid storageKey="m4.outbound.returns" columns={returnColumns} data={filteredReturns} rowKey={(row) => row.id} selectedKey={selectedId ?? undefined} onRowClick={(row) => setSelectedId(row.id)} emptyTitle="暂无采购退货单" />
      ) : (
        <DataGrid storageKey={`m4.outbound.${mode}`} columns={orderColumns} data={filteredOrders} rowKey={(row) => row.id} selectedKey={selectedId ?? undefined} onRowClick={(row) => setSelectedId(row.id)} emptyTitle="暂无出库单" />
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

function OrderActions({ row, mode, onDetail, onAction }: { row: OutboundOrder; mode: M4OutboundMode; onDetail: (id: string) => void; onAction: (kind: ActionKind, id: string) => void }) {
  if (mode === "review") {
    return <ActionButtons buttons={[["详情", Eye, () => onDetail(row.id)], ["复核", ClipboardCheck, () => onAction("review", row.id)], ["打印", Printer, () => onAction("print", row.id)], ["发货交接", Truck, () => onAction("ship", row.id)]]} />;
  }
  return <ActionButtons buttons={[["详情", Eye, () => onDetail(row.id)], ["重新校验", CheckCircle2, () => onAction("validate", row.id)], ["作废申请", ClipboardCheck, () => onAction("void", row.id)]]} />;
}

function WaveActions({ row, onDetail, onAction }: { row: OutboundWave; onDetail: (id: string) => void; onAction: (kind: ActionKind, id: string) => void }) {
  return <ActionButtons buttons={[["详情", Eye, () => onDetail(row.id)], ["下发", CheckCircle2, () => onAction("release-wave", row.id)], ["取消", ClipboardCheck, () => onAction("cancel-wave", row.id)]]} />;
}

function ReturnActions({ row, onDetail, onAction }: { row: PurchaseReturnOrder; onDetail: (id: string) => void; onAction: (kind: ActionKind, id: string) => void }) {
  return <ActionButtons buttons={[["详情", Eye, () => onDetail(row.id)], ["审批", CheckCircle2, () => onAction("approve-return", row.id)], ["拣货", Truck, () => onAction("pick-return", row.id)], ["复核", ClipboardCheck, () => onAction("review-return", row.id)], ["出库交接", CheckCircle2, () => onAction("ship-return", row.id)]]} />;
}

function ActionButtons({ buttons }: { buttons: Array<[string, React.ComponentType<{ className?: string }>, () => void]> }) {
  return (
    <div className="flex justify-end gap-2">
      {buttons.map(([label, Icon, onClick]) => (
        <Button key={label} type="button" variant="outline" size="sm" onClick={(event) => { event.stopPropagation(); onClick(); }}>
          <Icon className="size-4" aria-hidden />
          {label}
        </Button>
      ))}
    </div>
  );
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

function FilterBar({ keyword, statusFilter, dateFilter, mode, onKeywordChange, onStatusFilterChange, onDateFilterChange, onReset }: {
  keyword: string;
  statusFilter: string;
  dateFilter: string;
  mode: M4OutboundMode;
  onKeywordChange: (value: string) => void;
  onStatusFilterChange: (value: string) => void;
  onDateFilterChange: (value: string) => void;
  onReset: () => void;
}) {
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="grid gap-3 p-4 md:grid-cols-[minmax(18rem,1fr)_10rem_9rem_auto] md:items-end">
        <TextField label="关键字" value={keyword} onChange={onKeywordChange} placeholder={keywordPlaceholder(mode)} />
        <div>
          <label className="mb-1 block text-xs text-muted-foreground">状态</label>
          <select className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm" value={statusFilter} onChange={(event) => onStatusFilterChange(event.target.value)}>
            <option value="all">全部</option>
            {statusOptions(mode).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select>
        </div>
        <TextField label="日期" type="date" value={dateFilter} onChange={onDateFilterChange} />
        <Button type="button" variant="outline" onClick={onReset}>重置</Button>
      </CardContent>
    </Card>
  );
}

function TextField({ label, value, onChange, type = "text", placeholder, className }: { label: string; value: string; onChange: (value: string) => void; type?: string; placeholder?: string; className?: string }) {
  return (
    <label className={className}>
      <span className="mb-1 block text-xs text-muted-foreground">{label}</span>
      <Input type={type} value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function keywordPlaceholder(mode: M4OutboundMode) {
  if (mode === "returns") return "采购退货单 / 原采购入库单 / 商品 / 供应商";
  return "单号 / 商品 / 批号 / 客户";
}

function Metric({ label, value, tone }: { label: string; value: number; tone: "primary" | "warning" | "success" | "muted" }) {
  const toneClass = { primary: "text-primary", warning: "text-wms-warning", success: "text-wms-success", muted: "text-foreground" }[tone];
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="p-4">
        <p className="text-xs font-medium text-muted-foreground">{label}</p>
        <p className={`mt-2 text-2xl font-semibold tracking-normal ${toneClass}`}>{value}</p>
      </CardContent>
    </Card>
  );
}

function ProductSummary({ order }: { order: OutboundOrder }) {
  const first = order.lines[0];
  return (
    <div className="text-sm">
      <div className="font-medium">{first?.product_code ?? "-"}</div>
      <div className="text-xs text-muted-foreground">
        {order.lines.length} 行 / {first?.batch_no ?? "-"} / {totalPlannedQty(order)} 件 / 校验结果 {validationResultLabel(order)}
      </div>
    </div>
  );
}

function ReviewSummary({ order }: { order: OutboundOrder }) {
  return (
    <TwoLine
      top="复核模式 包装站复核"
      bottom={`计划 ${totalPlannedQty(order)} 件 / 短拣标识 ${order.short_pick ? "是" : "否"}`}
    />
  );
}

function OrderNoSummary({ order }: { order: OutboundOrder }) {
  return <TwoLine top={order.wms_order_no} bottom={`${order.erp_order_no ?? "-"} / 销售出库`} />;
}

function TwoLine({ top, bottom }: { top: string; bottom: string }) {
  return (
    <div className="text-sm">
      <div className="font-medium text-primary">{top}</div>
      <div className="text-xs text-muted-foreground">{bottom}</div>
    </div>
  );
}

function purchaseReturnDocumentTypeLabel(value: PurchaseReturnOrder["document_type"]) {
  return value === "purchase_return_outbound" ? "采购退货出库" : value;
}

function ActionExtraFields({ kind }: { kind: ActionKind }) {
  return <>{extraActionFields(kind).map(([label, value]) => <StaticField key={label} label={label} defaultValue={value} />)}</>;
}

function extraActionFields(kind: ActionKind): Array<[string, string]> {
  if (kind === "release-wave" || kind === "create-wave") return [["路径策略", "S 型最短路径"], ["温区", "常温"], ["容量上限", "100 单 / 10000 件"]];
  if (kind === "review") return [["工位码", "PK-STATION-01"], ["实际复核数量", "按扫码累计"], ["短拣标识", "否"], ["复核人", "当前用户"]];
  if (kind === "ship") return [["配送方类型", "第三方快递"], ["包裹数量", "1"], ["车牌号", "沪A-12345"], ["装车温度", "冷链时必填"], ["签字", "交接双方签字"]];
  if (kind.includes("return")) return [
    ["单据类型", "采购退货出库"],
    ["原采购入库单", "ASN-M2-PC-0001"],
    ["供应商", "华东医药供应商"],
    ["退货原因", "供应商召回"],
    ["商品", "P-M4-001"],
    ["数量", "3 件"],
    ["审批来源", "purchase_return_approval"],
  ];
  return [["校验结果", "指定批号库存充足"], ["审批来源", "企业微信"]];
}

function StaticField({ label, defaultValue }: { label: string; defaultValue: string }) {
  return (
    <label>
      <span className="mb-1 block text-xs text-muted-foreground">{label}</span>
      <Input defaultValue={defaultValue} />
    </label>
  );
}

function pageMeta(mode: M4OutboundMode) {
  const metrics = {
    orders: [
      metric("待校验", (data: StateData) => countOrders(data.orders, "pending_validation"), "primary"),
      metric("校验异常", (data: StateData) => countOrders(data.orders, "validation_exception"), "warning"),
      metric("已确认", (data: StateData) => countOrders(data.orders, "confirmed"), "success"),
      metric("本页合计", (data: StateData) => data.orders.length, "muted"),
    ],
    waves: [
      metric("待下发", (data: StateData) => countWaves(data.waves, "draft"), "primary"),
      metric("库存锁定", (data: StateData) => countWaves(data.waves, "inventory_locked"), "warning"),
      metric("已下发", (data: StateData) => countWaves(data.waves, "released"), "success"),
      metric("本页合计", (data: StateData) => data.waves.length, "muted"),
    ],
    review: [
      metric("待复核", (data: StateData) => countOrders(data.orders, "inventory_locked"), "primary"),
      metric("待打印", (data: StateData) => countOrders(data.orders, "reviewed"), "warning"),
      metric("已发货", (data: StateData) => countOrders(data.orders, "shipped"), "success"),
      metric("本页合计", (data: StateData) => data.orders.length, "muted"),
    ],
    returns: [
      metric("待审批", (data: StateData) => countReturns(data.returns, "pending_approval"), "primary"),
      metric("拣货中", (data: StateData) => countReturns(data.returns, "picking"), "warning"),
      metric("已发货", (data: StateData) => countReturns(data.returns, "shipped"), "success"),
      metric("本页合计", (data: StateData) => data.returns.length, "muted"),
    ],
  } satisfies Record<M4OutboundMode, MetricMeta[]>;
  const map = {
    orders: { title: "M4 出库订单管理", subtitle: "订单校验 · 双单号 · 作废申请", createAction: "create-order" as const, createLabel: "新建出库单" },
    waves: { title: "M4 波次规划", subtitle: "波次合并 · 库存锁定 · 路径策略", createAction: "create-wave" as const, createLabel: "新建波次" },
    review: { title: "M4 复核发货", subtitle: "包装站复核 · 打印 · 发货交接", createAction: null, createLabel: "" },
    returns: { title: "M4 采购退货出库", subtitle: "退供应商申请 · 审批 · 拣货复核 · 出库交接", createAction: "create-return" as const, createLabel: "新建采购退货单" },
  };
  return { ...map[mode], metrics: metrics[mode] };
}

interface StateData { orders: OutboundOrder[]; waves: OutboundWave[]; returns: PurchaseReturnOrder[] }
interface MetricMeta { label: string; value: (data: StateData) => number; tone: "primary" | "warning" | "success" | "muted" }

function metric(label: string, value: (data: StateData) => number, tone: MetricMeta["tone"]): MetricMeta {
  return { label, value, tone };
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

function filterOrders(orders: OutboundOrder[], keyword: string, status: string, date: string, mode: M4OutboundMode) {
  const allowed = mode === "review" ? new Set(["inventory_locked", "reviewed", "shipped"]) : null;
  return orders.filter((order) => {
    const searchable = [order.wms_order_no, order.erp_order_no ?? "", order.status, ...order.lines.flatMap((line) => [line.product_code, line.batch_no])].join(" ").toLowerCase();
    return (!allowed || allowed.has(order.status)) && matches(searchable, keyword) && (status === "all" || order.status === status) && (!date || order.required_ship_at?.slice(0, 10) === date);
  });
}

function filterWaves(waves: OutboundWave[], keyword: string, status: string) {
  return waves.filter((wave) => matches(`${wave.wave_no} ${wave.status}`.toLowerCase(), keyword) && (status === "all" || wave.status === status));
}

function filterReturns(returns: PurchaseReturnOrder[], keyword: string, status: string) {
  return returns.filter((item) => matches(`${item.return_no} ${item.document_type} ${item.source_purchase_order_no} ${item.supplier_name} ${item.reason} ${item.product_code} ${item.approval_source}`.toLowerCase(), keyword) && (status === "all" || item.status === status));
}

function matches(searchable: string, keyword: string) {
  const normalized = keyword.trim().toLowerCase();
  return !normalized || searchable.includes(normalized);
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

function countOrders(orders: OutboundOrder[], status: string) {
  return orders.filter((order) => order.status === status).length;
}

function countWaves(waves: OutboundWave[], status: string) {
  return waves.filter((wave) => wave.status === status).length;
}

function countReturns(returns: PurchaseReturnOrder[], status: string) {
  return returns.filter((item) => item.status === status).length;
}

function totalPlannedQty(order: OutboundOrder) {
  return order.lines.reduce((sum, line) => sum + line.planned_qty, 0);
}

function validationResultLabel(order: OutboundOrder) {
  return order.status === "validation_exception" ? "异常" : "通过";
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
