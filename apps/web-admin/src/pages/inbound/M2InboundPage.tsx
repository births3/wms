import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataTable,
  PageHeader,
  StatusBadge,
  type DataTableColumn,
  type StatusKey,
} from "@wms/ui";
import { ArrowLeft, Ban, CheckCircle2, ClipboardCheck, Eye, PackageCheck, Plus, Printer, RefreshCw, Signature } from "lucide-react";

import {
  useCreateReceivingOrderMutation,
  useInspectReceivingOrderMutation,
  usePutawayReceivingOrderMutation,
  useReceiveReceivingOrderMutation,
  useReceivingOrderQuery,
  useReceivingOrdersQuery,
  useRejectReceivingOrderMutation,
  useSignReceivingOrderMutation,
  type CreateReceivingOrderRequest,
  type ReceivingOrder,
} from "@/features/inbound/inbound-queries";
import {
  M2InboundDialogs,
  isColdChainTemperatureControl,
  type CreateFormState,
  type InboundDialog,
  type InspectFormState,
  type PutawayFormState,
  type ReceiveFormState,
  type RejectFormState,
  type SignFormState,
} from "./M2InboundDialogs";
import { M2InboundDetailDialog } from "./M2InboundDetailDialog";
import { M2InboundFilterBar, type StatusFilter } from "./M2InboundFilterBar";

export type M2InboundMode = "receiving" | "inspecting" | "putaway";

interface M2InboundPageProps {
  mode: M2InboundMode;
  onBack: () => void;
}

const firstSignerId = "00000000-0000-0000-0000-000000000101";
const secondSignerId = "00000000-0000-0000-0000-000000000102";
const defaultWarehouseId = "00000000-0000-0000-0000-000000003001";
const defaultLocationId = "00000000-0000-0000-0000-000000000201";
const defaultLocationCode = "A-01-01";

export function M2InboundPage({ mode, onBack }: M2InboundPageProps) {
  const ordersQuery = useReceivingOrdersQuery();
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [keyword, setKeyword] = React.useState("");
  const [statusFilter, setStatusFilter] = React.useState<StatusFilter>(() => defaultStatusFilter(mode));
  const [arrivalDate, setArrivalDate] = React.useState("");
  const [activeDialog, setActiveDialog] = React.useState<InboundDialog | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [createForm, setCreateForm] = React.useState<CreateFormState>({
    receiptNo: "ASN-M2-PC-0002",
    supplierId: "00000000-0000-0000-0000-000000005001",
    warehouseId: defaultWarehouseId,
    expectedArrivalDate: "",
    productCode: "P-M2-002",
    batchNo: "BATCH-202607",
    expectedQty: "60",
    productionDate: "2026-02-01",
    expiryDate: "2028-02-01",
  });
  const [receiveForm, setReceiveForm] = React.useState<ReceiveFormState>({
    actualQty: "0",
    shortageQty: "0",
    rejectedQty: "0",
    temperature: "",
    temperatureControl: "常温",
    vehicleNo: "沪A-12345",
    origin: "上海配送中心",
    departureTime: "2026-06-27T08:00",
    arrivalTime: "2026-06-27T10:00",
    storageTime: "2026-06-27T10:15",
    transportMode: "",
    carrier: "华东冷链承运商",
    contactName: "张三",
    contactPhone: "13800000000",
    contactIdNo: "310101199001010000",
    sealChecked: "已核对",
    filingChecked: "已核对",
    deliveryQty: "0",
    batchQty: "0",
    secondReceiverId: secondSignerId,
    note: "",
  });
  const [rejectForm, setRejectForm] = React.useState<RejectFormState>({
    reason: "",
  });
  const [inspectForm, setInspectForm] = React.useState<InspectFormState>({
    batchNo: "",
    acceptedQty: "0",
    rejectedQty: "0",
    productionDate: "2026-01-01",
    expiryDate: "2028-01-01",
    qualityStatus: "qualified",
    traceCodes: "TC-M2-0001",
    appearanceCheck: "外观合格",
    packageCheck: "包装合格",
    instructionCheck: "说明书合格",
    labelCheck: "标签合格",
    note: "",
  });
  const [signForm, setSignForm] = React.useState<SignFormState>({
    firstSignerId,
    secondSignerId,
    dualRequired: true,
    strategyNote: "process=入库，node=验收，dual_scan",
    note: "",
  });
  const [putawayForm, setPutawayForm] = React.useState<PutawayFormState>({
    lpn: "LPN-M2-PC-0001",
    productCode: "",
    batchNo: "",
    qty: "0",
    recommendedLocation: "A-01-01 / A-01-02 / A-02-01",
    locationId: defaultLocationId,
    locationCode: defaultLocationCode,
    qualityStatus: "qualified",
    validationResult: "温区/色标/容量校验通过",
    note: "",
  });

  const createMutation = useCreateReceivingOrderMutation();
  const receiveMutation = useReceiveReceivingOrderMutation();
  const rejectMutation = useRejectReceivingOrderMutation();
  const inspectMutation = useInspectReceivingOrderMutation();
  const signMutation = useSignReceivingOrderMutation();
  const putawayMutation = usePutawayReceivingOrderMutation();

  const orders = React.useMemo(
    () => filterOrders(ordersQuery.data ?? [], keyword, statusFilter, arrivalDate),
    [ordersQuery.data, keyword, statusFilter, arrivalDate],
  );

  React.useEffect(() => {
    if (selectedId && orders.some((order) => order.id === selectedId)) return;
    setSelectedId(orders[0]?.id ?? null);
  }, [orders, selectedId]);

  React.useEffect(() => {
    setStatusFilter(defaultStatusFilter(mode));
    setKeyword("");
    setArrivalDate("");
    setLastEvent(null);
  }, [mode]);

  const selectedFromList = ordersQuery.data?.find((order) => order.id === selectedId) ?? null;
  const detailQuery = useReceivingOrderQuery(selectedId);
  const order = detailQuery.data ?? selectedFromList;
  const line = order?.lines[0];
  const currentProductTemperatureAttribute = productTemperatureAttribute(line?.product_code);
  const currentTemperatureControl = temperatureControlFromProductAttribute(currentProductTemperatureAttribute);
  const pending =
    createMutation.isPending ||
    receiveMutation.isPending ||
    rejectMutation.isPending ||
    inspectMutation.isPending ||
    signMutation.isPending ||
    putawayMutation.isPending;
  const error =
    createMutation.error ??
    receiveMutation.error ??
    rejectMutation.error ??
    inspectMutation.error ??
    signMutation.error ??
    putawayMutation.error ??
    ordersQuery.error ??
    detailQuery.error;

  React.useEffect(() => {
    if (!order) return;
    const qty = String(totalExpectedQty(order));
    const firstLine = order.lines[0];
    setReceiveForm((value) => ({ ...value, actualQty: qty, shortageQty: "0", rejectedQty: "0" }));
    setRejectForm({ reason: "" });
    setInspectForm((value) => ({
      ...value,
      batchNo: firstLine?.batch_no ?? "BATCH-202606",
      acceptedQty: qty,
      rejectedQty: "0",
      productionDate: firstLine?.production_date ?? "2026-01-01",
      expiryDate: firstLine?.expiry_date ?? "2028-01-01",
    }));
    setPutawayForm((value) => ({
      ...value,
      productCode: firstLine?.product_code ?? "",
      batchNo: firstLine?.batch_no ?? "BATCH-202606",
      qty,
    }));
  }, [order?.id]);

  const orderColumns: DataTableColumn<ReceivingOrder>[] = [
    {
      key: "receipt_no",
      header: "ASN / 入库单",
      mono: true,
      render: (row) => <span className="text-primary">{row.receipt_no}</span>,
    },
    {
      key: "product",
      header: "商品 / 数量",
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.lines[0]?.product_code ?? "-"}</div>
          <div className="text-xs text-muted-foreground">{totalExpectedQty(row)} 件</div>
        </div>
      ),
    },
    { key: "work_fields", header: workFieldHeader(mode), render: (row) => <WorkFieldSummary order={row} mode={mode} /> },
    { key: "expected_arrival_at", header: "预计到货", render: (row) => formatDateTime(row.expected_arrival_at) },
    {
      key: "status",
      header: "状态",
      render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
    },
    {
      key: "actions",
      header: "操作",
      align: "right",
      render: (row) => (
        <div className="flex justify-end gap-2">
          <RowButton
            icon={<Eye className="size-4" aria-hidden />}
            label="详情"
            onClick={() => openRowDetail(row.id)}
          />
          {mode === "receiving" && canReceiveOrReject(row.status) && (
            <>
              <RowButton
                icon={<CheckCircle2 className="size-4" aria-hidden />}
                label="收货"
                onClick={() => openRowDialog(row.id, "receive")}
              />
              <RowButton
                icon={<Ban className="size-4" aria-hidden />}
                label="拒收"
                onClick={() => openRowDialog(row.id, "reject")}
              />
            </>
          )}
          {mode === "inspecting" && (
            <>
              <RowButton
                icon={<ClipboardCheck className="size-4" aria-hidden />}
                label="验收"
                onClick={() => openRowDialog(row.id, "inspect")}
              />
              <RowButton
                icon={<Signature className="size-4" aria-hidden />}
                label="双人签字"
                onClick={() => openRowDialog(row.id, "sign")}
              />
            </>
          )}
          {mode === "putaway" && (
            <RowButton
              icon={<PackageCheck className="size-4" aria-hidden />}
              label="上架"
              onClick={() => openRowDialog(row.id, "putaway")}
            />
          )}
        </div>
      ),
    },
  ];

  async function refreshInbound() {
    await ordersQuery.refetch();
    if (selectedId) await detailQuery.refetch();
    setLastEvent("入库列表已刷新");
  }

  function openRowDetail(id: string) {
    setSelectedId(id);
    setDetailOpen(true);
  }

  function openRowDialog(id: string, dialog: InboundDialog) {
    setSelectedId(id);
    setActiveDialog(dialog);
  }

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const request: CreateReceivingOrderRequest = {
      receipt_no: createForm.receiptNo.trim(),
      warehouse_id: createForm.warehouseId.trim(),
      expected_arrival_at: dateToIso(createForm.expectedArrivalDate),
      external_ref: null,
      supplier_id: createForm.supplierId.trim() || null,
      lines: [
        {
          line_no: 1,
          product_code: createForm.productCode.trim(),
          product_id: null,
          batch_no: createForm.batchNo.trim() || null,
          expected_qty: toInteger(createForm.expectedQty),
          production_date: createForm.productionDate || null,
          expiry_date: createForm.expiryDate || null,
        },
      ],
    };
    const created = await createMutation.mutateAsync(request);
    setSelectedId(created.id);
    setActiveDialog(null);
    setLastEvent(`${created.receipt_no} 已创建`);
  }

  async function submitReceive(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    const coldChain = isColdChainTemperatureControl(currentTemperatureControl);
    await receiveMutation.mutateAsync({
      id: order.id,
      request: {
        actual_qty: toInteger(receiveForm.actualQty),
        shortage_qty: toInteger(receiveForm.shortageQty),
        rejected_qty: toInteger(receiveForm.rejectedQty),
        arrival_temperature_celsius: coldChain && receiveForm.temperature !== "" ? Number(receiveForm.temperature) : null,
        exception_note: receiveForm.note.trim() || null,
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 收货已提交`);
  }

  async function submitReject(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    await rejectMutation.mutateAsync({
      id: order.id,
      request: {
        reason: rejectForm.reason.trim(),
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 已关闭(拒收)`);
  }

  async function submitInspect(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    await inspectMutation.mutateAsync({
      id: order.id,
      request: {
        batch_no: inspectForm.batchNo.trim() || line?.batch_no || "BATCH-202606",
        accepted_qty: toInteger(inspectForm.acceptedQty),
        rejected_qty: toInteger(inspectForm.rejectedQty),
        production_date: inspectForm.productionDate,
        expiry_date: inspectForm.expiryDate,
        quality_status: inspectForm.qualityStatus.trim(),
        trace_codes: splitCodes(inspectForm.traceCodes || "TC-M2-0001"),
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 验收已提交`);
  }

  async function submitSign(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    await signMutation.mutateAsync({
      id: order.id,
      request: {
        first_signer_id: signForm.firstSignerId.trim(),
        second_signer_id: signForm.dualRequired ? signForm.secondSignerId.trim() : null,
        dual_required: signForm.dualRequired,
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 双人签字已提交`);
  }

  async function submitPutaway(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    await putawayMutation.mutateAsync({
      id: order.id,
      request: {
        product_code: putawayForm.productCode.trim(),
        batch_no: putawayForm.batchNo.trim() || line?.batch_no || "BATCH-202606",
        qty: toInteger(putawayForm.qty),
        location_id: putawayForm.locationId.trim(),
        location_code: putawayForm.locationCode.trim(),
        quality_status: putawayForm.qualityStatus.trim(),
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 上架已提交`);
  }

  const pageMeta = inboundPageMeta(mode);

  return (
    <section className="mx-auto flex w-full max-w-[1400px] flex-col gap-5 px-6 py-8">
        <PageHeader
          title={pageMeta.title}
          subtitle={pageMeta.subtitle}
          actions={
            <div className="flex flex-wrap gap-2">
              {lastEvent && (
                <span className="self-center text-sm text-muted-foreground" role="status">
                  {lastEvent}
                </span>
              )}
              <Button type="button" variant="outline" onClick={refreshInbound}>
                <RefreshCw className="size-4" aria-hidden />
                刷新
              </Button>
              <Button type="button" variant="outline" onClick={() => globalThis.print()}>
                <Printer className="size-4" aria-hidden />
                打印
              </Button>
              {mode === "receiving" && (
                <Button type="button" onClick={() => setActiveDialog("create")}>
                  <Plus className="size-4" aria-hidden />
                  新建 ASN
                </Button>
              )}
              <Button type="button" variant="outline" onClick={onBack}>
                <ArrowLeft className="size-4" aria-hidden />
                返回工作台
              </Button>
            </div>
          }
        />

        <div className="grid gap-3 md:grid-cols-4">
          <Metric label="待处理" value={countByStatus(ordersQuery.data ?? [], "receiving")} tone="primary" />
          <Metric label="验收中" value={countByStatus(ordersQuery.data ?? [], "inspecting")} tone="warning" />
          <Metric label="上架中" value={countByStatus(ordersQuery.data ?? [], "putaway")} tone="success" />
          <Metric label="本页合计" value={(ordersQuery.data ?? []).length} tone="muted" />
        </div>

        <M2InboundFilterBar
          keyword={keyword}
          statusFilter={statusFilter}
          arrivalDate={arrivalDate}
          onKeywordChange={setKeyword}
          onStatusFilterChange={setStatusFilter}
          onArrivalDateChange={setArrivalDate}
          onReset={() => {
            setKeyword("");
            setStatusFilter("all");
            setArrivalDate("");
          }}
        />

        {error && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error.message}
          </div>
        )}

        <DataTable
          columns={orderColumns}
          data={orders}
          rowKey={(row) => row.id}
          selectedKey={selectedId ?? undefined}
          onRowClick={(row) => setSelectedId(row.id)}
          caption={ordersQuery.isPending ? "加载入库单..." : `共 ${orders.length} 张入库单，点击行选择，操作按钮弹窗处理`}
          emptyTitle="暂无入库单"
        />

        <M2InboundDialogs
          activeDialog={activeDialog}
          orderReceiptNo={order?.receipt_no ?? null}
          hasOrder={Boolean(order)}
          pending={pending}
          errorMessage={error?.message}
          productTemperatureAttribute={currentProductTemperatureAttribute}
          derivedTemperatureControl={currentTemperatureControl}
          createForm={createForm}
          receiveForm={receiveForm}
          rejectForm={rejectForm}
          inspectForm={inspectForm}
          signForm={signForm}
          putawayForm={putawayForm}
          setActiveDialog={setActiveDialog}
          setCreateForm={setCreateForm}
          setReceiveForm={setReceiveForm}
          setRejectForm={setRejectForm}
          setInspectForm={setInspectForm}
          setSignForm={setSignForm}
          setPutawayForm={setPutawayForm}
          submitCreate={submitCreate}
          submitReceive={submitReceive}
          submitReject={submitReject}
          submitInspect={submitInspect}
          submitSign={submitSign}
          submitPutaway={submitPutaway}
        />
        <M2InboundDetailDialog order={order} defaultStage={detailStageFromMode(mode)} open={detailOpen} onOpenChange={setDetailOpen} />
    </section>
  );
}

function RowButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
    >
      {icon}
      {label}
    </Button>
  );
}

function Metric({ label, value, tone }: { label: string; value: number; tone: "primary" | "warning" | "success" | "muted" }) {
  const toneClass = {
    primary: "text-primary",
    warning: "text-wms-warning",
    success: "text-wms-success",
    muted: "text-foreground",
  }[tone];
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="p-4">
        <p className="text-xs font-medium text-muted-foreground">{label}</p>
        <p className={`mt-2 text-2xl font-semibold tracking-normal ${toneClass}`}>{value}</p>
      </CardContent>
    </Card>
  );
}

function WorkFieldSummary({ order, mode }: { order: ReceivingOrder; mode: M2InboundMode }) {
  const line = order.lines[0];
  const content = {
    receiving: [`供应商 ${shortId(order.supplier_id ?? "00000000")}`, "承运商 华东冷链 / 车牌沪A-12345"],
    inspecting: [`批号 ${line?.batch_no ?? "-"}`, `效期 ${line?.expiry_date ?? "-"} / 质量待验`],
    putaway: [`推荐 A-01-01 / 实际待录入`, "LPN-M2-PC-0001 / 校验待执行"],
  }[mode];
  return (
    <div className="text-sm">
      <div className="font-medium">{content[0]}</div>
      <div className="text-xs text-muted-foreground">{content[1]}</div>
    </div>
  );
}

function workFieldHeader(mode: M2InboundMode) {
  const headers: Record<M2InboundMode, string> = {
    receiving: "供应商 / 承运",
    inspecting: "批号 / 质量",
    putaway: "库位 / LPN",
  };
  return headers[mode];
}

function filterOrders(orders: ReceivingOrder[], keyword: string, statusFilter: StatusFilter, arrivalDate: string) {
  const normalized = keyword.trim().toLowerCase();
  return orders.filter((order) => {
    const matchesStatus = matchesStatusFilter(order.status, statusFilter);
    const matchesDate = !arrivalDate || order.expected_arrival_at?.slice(0, 10) === arrivalDate;
    const searchable = [order.receipt_no, order.status, ...order.lines.flatMap((line) => [line.product_code, line.batch_no ?? ""])]
      .join(" ")
      .toLowerCase();
    return matchesStatus && matchesDate && (!normalized || searchable.includes(normalized));
  });
}

function defaultStatusFilter(mode: M2InboundMode): StatusFilter {
  return mode;
}

function detailStageFromMode(mode: M2InboundMode) {
  const map = { receiving: "receiving", inspecting: "inspection", putaway: "putaway" } as const;
  return map[mode];
}

function inboundPageMeta(mode: M2InboundMode) {
  const meta: Record<M2InboundMode, { title: string; subtitle: string }> = {
    receiving: {
      title: "M2 收货管理",
      subtitle: "ASN 接收 · 到货登记 · 实到/缺货/拒收",
    },
    inspecting: {
      title: "M2 验收管理",
      subtitle: "批号效期验收 · 追溯码 · 双人签字",
    },
    putaway: {
      title: "M2 上架管理",
      subtitle: "库位确认 · 商品批号 · 数量上架",
    },
  };
  return meta[mode];
}

function countByStatus(orders: ReceivingOrder[], status: string) {
  return orders.filter((order) => (status === "receiving" ? canReceiveOrReject(order.status) : order.status === status)).length;
}

function matchesStatusFilter(status: string, filter: StatusFilter) {
  if (filter === "all") return true;
  if (filter === "receiving") return canReceiveOrReject(status);
  return status === filter;
}

function canReceiveOrReject(status: string) {
  return status === "released" || status === "receiving";
}

function statusKey(status: string): StatusKey {
  if (status === "completed") return "completed";
  if (status.includes("receiv") || status.includes("inspect") || status.includes("putaway")) return "in_progress";
  if (status.includes("reject") || status.includes("closed")) return "unqualified";
  return "pending";
}

function statusLabel(status: string) {
  const labels: Record<string, string> = {
    pending: "待处理",
    released: "待收货",
    receiving: "收货中",
    inspecting: "验收中",
    putaway: "上架中",
    completed: "已完成",
    closed_rejected: "已关闭(拒收)",
  };
  return labels[status] ?? status;
}

function totalExpectedQty(order: ReceivingOrder) {
  return order.lines.reduce((sum, line) => sum + line.expected_qty, 0);
}

function productTemperatureAttribute(productCode: string | null | undefined) {
  // ponytail: ReceivingOrderLine 还没有商品温度属性；后端补字段后替换这里。
  if (!productCode) return "常温";
  if (/冻|FROZEN/i.test(productCode)) return "冷冻";
  if (/冷|COLD|P-M2-002/i.test(productCode)) return "冷藏";
  return "常温";
}

function temperatureControlFromProductAttribute(attribute: string) {
  if (attribute === "冷冻") return "冷冻车";
  if (attribute === "冷藏") return "冷藏车";
  return "常温";
}

function toInteger(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function splitCodes(value: string) {
  return value.split(/\s+/).map((item) => item.trim()).filter(Boolean);
}

function dateToIso(value: string) {
  return value ? `${value}T10:00:00.000Z` : null;
}

function shortId(value: string) {
  return value.slice(0, 8);
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("zh-CN", { hour12: false });
}
