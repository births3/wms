import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  DataGrid,
  PageHeader,
  StatusBadge,
  type DataGridColumn,
} from "@wms/ui";
import { ArrowLeft, CheckCircle2, ClipboardCheck, Eye, PackageCheck, Plus, Printer, RefreshCw } from "lucide-react";

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
import {
  inboundDocumentTypeLabel,
  inboundDocumentTypeOf,
  createAsnBatchNo,
  type InboundDocumentTypeFilter,
} from "./m2-inbound-document-type";
import {
  canReceiveOrReject,
  countByStatus,
  dateToIso,
  defaultCreatedDateRange,
  defaultStatusFilter,
  detailStageFromMode,
  filterOrders,
  formatDateTime,
  inboundPageMeta,
  ownerLabel,
  productTemperatureAttribute,
  shortId,
  splitCodes,
  statusKey,
  statusLabel,
  temperatureControlFromProductAttribute,
  toInteger,
  totalExpectedQty,
  workFieldHeader,
  workFieldText,
  type M2InboundMode,
  type OwnerContext,
} from "./m2-inbound-page-helpers";
import { M2InboundFilterBar, type StatusFilter } from "./M2InboundFilterBar";

export type { M2InboundMode } from "./m2-inbound-page-helpers";

interface M2InboundPageProps {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  onBack: () => void;
}

const firstSignerId = "00000000-0000-0000-0000-000000000101";
const secondSignerId = "00000000-0000-0000-0000-000000000102";
const defaultWarehouseId = "00000000-0000-0000-0000-000000003001";
const defaultLocationId = "00000000-0000-0000-0000-000000000201";
const defaultLocationCode = "A-01-01";

export function M2InboundPage({ mode, currentOwner, onBack }: M2InboundPageProps) {
  const ordersQuery = useReceivingOrdersQuery();
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [keyword, setKeyword] = React.useState("");
  const [ownerKeyword, setOwnerKeyword] = React.useState(currentOwner.ownerCode);
  const [documentTypeFilter, setDocumentTypeFilter] = React.useState<InboundDocumentTypeFilter>([]);
  const [statusFilter, setStatusFilter] = React.useState<StatusFilter>(() => defaultStatusFilter(mode));
  const [arrivalDate, setArrivalDate] = React.useState("");
  const [createdAtFrom, setCreatedAtFrom] = React.useState(() => defaultCreatedDateRange().from);
  const [createdAtTo, setCreatedAtTo] = React.useState(() => defaultCreatedDateRange().to);
  const [activeDialog, setActiveDialog] = React.useState<InboundDialog | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [createForm, setCreateForm] = React.useState<CreateFormState>({
    receiptNo: "ASN-M2-PC-0002",
    documentType: "purchase_inbound",
    supplierId: "00000000-0000-0000-0000-000000005001",
    warehouseId: defaultWarehouseId,
    expectedArrivalDate: "",
    productCode: "P-M2-002",
    batchNo: "",
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
    () =>
      filterOrders(
        ordersQuery.data ?? [],
        keyword,
        documentTypeFilter,
        statusFilter,
        arrivalDate,
        createdAtFrom,
        createdAtTo,
        ownerKeyword,
        currentOwner,
      ),
    [ordersQuery.data, keyword, documentTypeFilter, statusFilter, arrivalDate, createdAtFrom, createdAtTo, ownerKeyword, currentOwner],
  );

  React.useEffect(() => {
    if (selectedId && orders.some((order) => order.id === selectedId)) return;
    setSelectedId(orders[0]?.id ?? null);
  }, [orders, selectedId]);

  React.useEffect(() => {
    setStatusFilter(defaultStatusFilter(mode));
    setKeyword("");
    setOwnerKeyword(currentOwner.ownerCode);
    setDocumentTypeFilter([]);
    setArrivalDate("");
    resetCreatedDateRange();
    setLastEvent(null);
  }, [mode, currentOwner.ownerCode]);

  function resetCreatedDateRange() {
    const range = defaultCreatedDateRange();
    setCreatedAtFrom(range.from);
    setCreatedAtTo(range.to);
  }

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

  const orderColumns: DataGridColumn<ReceivingOrder>[] = [
    {
      key: "receipt_no",
      header: "ASN / 入库单",
      mono: true,
      width: 220,
      minWidth: 210,
      sortable: true,
      sortValue: (row) => row.receipt_no,
      filterValue: (row) => row.receipt_no,
      copyValue: (row) => row.receipt_no,
      filter: { type: "text" },
      onDoubleClick: (row) => openRowDetail(row.id),
      render: (row) => <span className="text-primary">{row.receipt_no}</span>,
    },
    {
      key: "owner",
      header: "货主",
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => ownerLabel(row.owner_id, currentOwner),
      filterValue: (row) => [row.owner_id, ownerLabel(row.owner_id, currentOwner)].join(" "),
      copyValue: (row) => ownerLabel(row.owner_id, currentOwner),
      filter: { type: "text" },
      render: (row) => ownerLabel(row.owner_id, currentOwner),
    },
    {
      key: "document_type",
      header: "单据类型",
      width: 150,
      minWidth: 140,
      sortable: true,
      sortValue: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
      filterValue: (row) => inboundDocumentTypeOf(row),
      copyValue: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
      filter: {
        type: "multiSelect",
        options: [
          { label: "采购入库", value: "purchase_inbound" },
          { label: "销售退货", value: "sales_return" },
        ],
      },
      render: (row) => inboundDocumentTypeLabel(inboundDocumentTypeOf(row)),
    },
    {
      key: "product",
      header: "商品 / 数量",
      width: 200,
      minWidth: 180,
      copyValue: (row) => {
        const line = row.lines[0];
        return `${line?.product_code ?? "-"} ${totalExpectedQty(row)} 件`;
      },
      filterValue: (row) => {
        const line = row.lines[0];
        return [line?.product_code ?? "", line?.batch_no ?? "", totalExpectedQty(row)].join(" ");
      },
      filter: { type: "text" },
      render: (row) => (
        <div className="text-sm">
          <div className="font-medium">{row.lines[0]?.product_code ?? "-"}</div>
          <div className="text-xs text-muted-foreground">{totalExpectedQty(row)} 件</div>
        </div>
      ),
    },
    {
      key: "work_fields",
      header: workFieldHeader(mode),
      width: 440,
      minWidth: 360,
      filterValue: (row) => workFieldText(row, mode),
      copyValue: (row) => workFieldText(row, mode),
      filter: { type: "text" },
      render: (row) => <WorkFieldSummary order={row} mode={mode} />,
    },
    {
      key: "expected_arrival_at",
      header: "预计到货",
      width: 190,
      minWidth: 180,
      sortable: true,
      sortValue: (row) => row.expected_arrival_at ?? "",
      filterValue: (row) => row.expected_arrival_at,
      copyValue: (row) => formatDateTime(row.expected_arrival_at),
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.expected_arrival_at),
    },
    {
      key: "status",
      header: "状态",
      width: 170,
      minWidth: 150,
      sortable: true,
      sortValue: (row) => statusLabel(row.status),
      filterValue: (row) => row.status,
      copyValue: (row) => statusLabel(row.status),
      filter: {
        type: "multiSelect",
        options: [
          { label: "待处理", value: "pending" },
          { label: "待收货", value: "released" },
          { label: "收货中", value: "receiving" },
          { label: "验收中", value: "inspecting" },
          { label: "上架中", value: "putaway" },
          { label: "已完成", value: "completed" },
          { label: "已关闭(拒收)", value: "closed_rejected" },
        ],
      },
      render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
    },
    {
      key: "actions",
      header: "操作",
      align: "right",
      width: 230,
      minWidth: 220,
      hideable: false,
      copyable: false,
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
            </>
          )}
          {mode === "inspecting" && (
            <RowButton
              icon={<ClipboardCheck className="size-4" aria-hidden />}
              label="验收"
              onClick={() => openRowDialog(row.id, "inspect")}
            />
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

  async function refreshInbound(message = "入库列表已刷新") {
    await ordersQuery.refetch();
    if (selectedId) await detailQuery.refetch();
    setLastEvent(message);
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
      document_type: createForm.documentType,
      warehouse_id: createForm.warehouseId.trim(),
      expected_arrival_at: dateToIso(createForm.expectedArrivalDate),
      external_ref: null,
      supplier_id: createForm.supplierId.trim() || null,
      lines: [
        {
          line_no: 1,
          product_code: createForm.productCode.trim(),
          product_id: null,
          batch_no: createAsnBatchNo(createForm.documentType, createForm.batchNo),
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

  async function submitReject(event?: React.FormEvent<HTMLFormElement>) {
    event?.preventDefault();
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
    await signMutation.mutateAsync({
      id: order.id,
      request: {
        first_signer_id: signForm.firstSignerId.trim(),
        second_signer_id: signForm.dualRequired ? signForm.secondSignerId.trim() : null,
        dual_required: signForm.dualRequired,
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 验收已提交`);
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
    <section className="mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 py-8 xl:px-6">
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
              <Button type="button" variant="outline" onClick={() => refreshInbound()}>
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
          ownerKeyword={ownerKeyword}
          documentTypeFilter={documentTypeFilter}
          statusFilter={statusFilter}
          arrivalDate={arrivalDate}
          createdAtFrom={createdAtFrom}
          createdAtTo={createdAtTo}
          onKeywordChange={setKeyword}
          onOwnerKeywordChange={setOwnerKeyword}
          onDocumentTypeFilterChange={setDocumentTypeFilter}
          onStatusFilterChange={setStatusFilter}
          onArrivalDateChange={setArrivalDate}
          onCreatedAtFromChange={setCreatedAtFrom}
          onCreatedAtToChange={setCreatedAtTo}
          onQuery={() => refreshInbound("入库列表已查询")}
          onReset={() => {
            setKeyword("");
            setOwnerKeyword(currentOwner.ownerCode);
            setDocumentTypeFilter([]);
            setStatusFilter([]);
            setArrivalDate("");
            resetCreatedDateRange();
          }}
        />

        {error && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error.message}
          </div>
        )}

        <DataGrid
          columns={orderColumns}
          data={orders}
          rowKey={(row) => row.id}
          selectedKey={selectedId ?? undefined}
          onRowClick={(row) => setSelectedId(row.id)}
          caption={ordersQuery.isPending ? "加载入库单..." : undefined}
          emptyTitle="暂无入库单"
          storageKey="m2-inbound-datagrid"
          tableClassName="min-w-[1880px]"
          selectable
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
          submitPutaway={submitPutaway}
        />
        <M2InboundDetailDialog
          order={order}
          currentOwner={currentOwner}
          defaultStage={detailStageFromMode(mode)}
          open={detailOpen}
          onOpenChange={setDetailOpen}
        />
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
  const content = workFieldText(order, mode);
  return (
    <div className="text-sm">
      <div className="font-medium">{content[0]}</div>
      <div className="text-xs text-muted-foreground">{content[1]}</div>
    </div>
  );
}
