import * as React from "react";
import {
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";

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
} from "@/features/inbound/inbound-queries";
import {
  M2InboundDialogs,
  isColdChainTemperatureControl,
  type CreateFormState,
  type InboundDialog,
  type InspectFormExamples,
  type InspectFormState,
  type PutawayFormState,
  type ReceiveFormState,
  type RejectFormState,
  type SignFormState,
} from "./M2InboundDialogs";
import { M2InboundDetailDialog } from "./M2InboundDetailDialog";
import {
  createAsnBatchNo,
  type InboundDocumentTypeFilter,
} from "./m2-inbound-document-type";
import {
  dateToIso,
  defaultCreatedDateRange,
  defaultStatusFilter,
  detailStageFromMode,
  filterOrders,
  inboundPageMeta,
  productTemperatureAttribute,
  splitCodes,
  temperatureControlFromProductAttribute,
  toInteger,
  totalExpectedQty,
  type M2InboundQueryValue,
  type M2InboundMode,
  type OwnerContext,
  type StatusFilter,
} from "./m2-inbound-page-helpers";
import { M2InboundOrderTable } from "./M2InboundOrderTable";

export type { M2InboundMode } from "./m2-inbound-page-helpers";

interface M2InboundPageProps {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  onBack: () => void;
}

const firstSignerId = "00000000-0000-0000-0000-000000000101";
const secondSignerId = "00000000-0000-0000-0000-000000000102";
const defaultLocationId = "00000000-0000-0000-0000-000000000201";
const defaultLocationCode = "A-01-01";
const emptyCreateForm: CreateFormState = {
  receiptNo: "",
  documentType: "",
  supplierId: "",
  warehouseId: "",
  expectedArrivalDate: "",
  productCode: "",
  batchNo: "",
  expectedQty: "",
  productionDate: "",
  expiryDate: "",
};
const emptyInspectForm: InspectFormState = {
  batchNo: "",
  acceptedQty: "",
  rejectedQty: "",
  productionDate: "",
  expiryDate: "",
  qualityStatus: "",
  traceCodes: "",
  appearanceCheck: "",
  packageCheck: "",
  instructionCheck: "",
  labelCheck: "",
  note: "",
};
const emptySignForm: SignFormState = {
  firstSignerId: "",
  secondSignerId: "",
  dualRequired: true,
  strategyNote: "",
  note: "",
};
const m2InboundQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "ASN / 商品 / 批号 / 单据类型",
  },
  {
    key: "ownerKeyword",
    label: "货主",
    type: "text",
    placeholder: "货主编码 / ID",
  },
  {
    key: "documentTypeFilter",
    label: "单据类型",
    type: "multiSelect",
    options: [
      { value: "purchase_inbound", label: "采购入库" },
      { value: "sales_return", label: "销售退货" },
    ],
  },
  {
    key: "statusFilter",
    label: "状态",
    type: "multiSelect",
    options: [
      { value: "receiving", label: "待收货/收货中" },
      { value: "inspecting", label: "验收中" },
      { value: "putaway", label: "上架中" },
      { value: "completed", label: "已完成" },
      { value: "closed_rejected", label: "已关闭(拒收)" },
    ],
  },
  {
    key: "arrivalDate",
    label: "预计到货",
    type: "dateRange",
  },
  {
    key: "createdAt",
    label: "创建时间",
    type: "dateRange",
  },
];
const m2InboundCoreQueryFieldKeys = ["keyword", "ownerKeyword", "statusFilter"];

export function M2InboundPage({ mode, currentOwner }: M2InboundPageProps) {
  const ordersQuery = useReceivingOrdersQuery();
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const defaultQuery = React.useMemo(
    () => defaultM2InboundQueryValue(mode, currentOwner),
    [mode, currentOwner.ownerCode, currentOwner.ownerId],
  );
  const [draftQuery, setDraftQuery] = React.useState<M2InboundQueryValue>(() =>
    defaultM2InboundQueryValue(mode, currentOwner),
  );
  const [appliedQuery, setAppliedQuery] = React.useState<M2InboundQueryValue>(() =>
    defaultM2InboundQueryValue(mode, currentOwner),
  );
  const [activeDialog, setActiveDialog] = React.useState<InboundDialog | null>(null);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [createForm, setCreateForm] = React.useState<CreateFormState>(emptyCreateForm);
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
  const [inspectForm, setInspectForm] = React.useState<InspectFormState>(emptyInspectForm);
  const [signForm, setSignForm] = React.useState<SignFormState>(emptySignForm);
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
        appliedQuery.keyword,
        appliedQuery.documentTypeFilter,
        appliedQuery.statusFilter,
        appliedQuery.arrivalDate.from ?? "",
        appliedQuery.arrivalDate.to ?? "",
        appliedQuery.createdAt.from ?? "",
        appliedQuery.createdAt.to ?? "",
        appliedQuery.ownerKeyword,
        currentOwner,
      ),
    [ordersQuery.data, appliedQuery, currentOwner],
  );
  const m2QuerySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(m2InboundQueryFields, appliedQuery),
    [appliedQuery],
  );

  React.useEffect(() => {
    if (selectedId && orders.some((order) => order.id === selectedId)) return;
    setSelectedId(orders[0]?.id ?? null);
  }, [orders, selectedId]);

  React.useEffect(() => {
    const next = defaultM2InboundQueryValue(mode, currentOwner);
    setDraftQuery(next);
    setAppliedQuery(next);
    setLastEvent(null);
  }, [mode, currentOwner.ownerCode, currentOwner.ownerId]);

  const selectedFromList = ordersQuery.data?.find((order) => order.id === selectedId) ?? null;
  const detailQuery = useReceivingOrderQuery(selectedId);
  const order = detailQuery.data ?? selectedFromList;
  const line = order?.lines[0];
  const totalQty = order ? String(totalExpectedQty(order)) : "";
  const inspectExamples: InspectFormExamples = {
    batchNo: exampleText(line?.batch_no, "请输入验收批号"),
    acceptedQty: exampleText(totalQty, "请输入通过数量"),
    rejectedQty: "例如 0",
    productionDate: exampleText(line?.production_date, "请选择生产日期"),
    expiryDate: exampleText(line?.expiry_date, "请选择有效期"),
    traceCodes: "例如 TC-M2-0001",
    appearanceCheck: "例如 外观合格",
    packageCheck: "例如 包装合格",
    instructionCheck: "例如 说明书合格",
    labelCheck: "例如 标签合格",
    firstSignerId: `例如 ${firstSignerId}`,
    secondSignerId: `例如 ${secondSignerId}`,
    strategyNote: "例如 process=入库，node=验收，dual_scan",
  };
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
    setReceiveForm((value) => ({ ...value, actualQty: qty, shortageQty: "0", rejectedQty: "0" }));
    setRejectForm({ reason: "" });
    setInspectForm(emptyInspectForm);
    setSignForm(emptySignForm);
    setPutawayForm((value) => ({
      ...value,
      productCode: order.lines[0]?.product_code ?? "",
      batchNo: order.lines[0]?.batch_no ?? "BATCH-202606",
      qty,
    }));
  }, [order?.id]);

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
    if (dialog === "inspect") {
      setInspectForm(emptyInspectForm);
      setSignForm(emptySignForm);
    }
    setActiveDialog(dialog);
  }

  function openCreateDialog() {
    setCreateForm(emptyCreateForm);
    setActiveDialog("create");
  }

  async function submitCreate(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!createForm.documentType) return;
    const documentType = createForm.documentType;
    const request: CreateReceivingOrderRequest = {
      receipt_no: createForm.receiptNo.trim(),
      document_type: documentType,
      warehouse_id: createForm.warehouseId.trim(),
      expected_arrival_at: dateToIso(createForm.expectedArrivalDate),
      external_ref: null,
      supplier_id: createForm.supplierId.trim() || null,
      lines: [
        {
          line_no: 1,
          product_code: createForm.productCode.trim(),
          product_id: null,
          batch_no: createAsnBatchNo(documentType, createForm.batchNo),
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
        batch_no: inspectForm.batchNo.trim(),
        accepted_qty: toInteger(inspectForm.acceptedQty),
        rejected_qty: toInteger(inspectForm.rejectedQty),
        production_date: inspectForm.productionDate,
        expiry_date: inspectForm.expiryDate,
        quality_status: inspectForm.qualityStatus.trim(),
        trace_codes: splitCodes(inspectForm.traceCodes),
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
  const tableRefreshAction = {
    label: "刷新",
    onClick: () => {
      void refreshInbound();
    },
  };
  const tableCreateAction =
    mode === "receiving"
      ? {
          label: "新增",
          description: "新建 ASN",
          onClick: openCreateDialog,
        }
      : undefined;

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
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
            </div>
          }
        />

        <QueryPanel
          fields={m2InboundQueryFields}
          defaultVisibleFieldKeys={m2InboundCoreQueryFieldKeys}
          value={draftQuery}
          onValueChange={(next) => setDraftQuery(normalizeM2InboundQueryValue(next, defaultQuery))}
          onQuery={() => {
            setAppliedQuery(normalizeM2InboundQueryValue(draftQuery, defaultQuery));
            void refreshInbound("入库列表已查询");
          }}
          onReset={() => {
            const next = defaultM2InboundQueryValue(mode, currentOwner);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
        />

        {error && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error.message}
          </div>
        )}

        <M2InboundOrderTable
          mode={mode}
          currentOwner={currentOwner}
          orders={orders}
          exportFileBaseName={pageMeta.title}
          selectedId={selectedId}
          isPending={ordersQuery.isPending}
          onSelectOrder={setSelectedId}
          onOpenDetail={openRowDetail}
          onOpenDialog={openRowDialog}
          refreshAction={tableRefreshAction}
          createAction={tableCreateAction}
          queryState={appliedQuery}
          querySummaryItems={m2QuerySummaryItems}
          onApplyQueryState={(queryState) => {
            const next = normalizeM2InboundQueryValue(queryValueFromUnknown(queryState), defaultQuery);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
          onClearQueryState={() => {
            const next = defaultM2InboundQueryValue(mode, currentOwner);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectedId(null);
          }}
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
          inspectExamples={inspectExamples}
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

function defaultM2InboundQueryValue(
  mode: M2InboundMode,
  currentOwner: OwnerContext,
): M2InboundQueryValue {
  const createdAt = defaultCreatedDateRange();
  return {
    keyword: "",
    ownerKeyword: currentOwner.ownerCode,
    documentTypeFilter: [],
    statusFilter: defaultStatusFilter(mode),
    arrivalDate: { from: "", to: "" },
    createdAt,
  };
}

function normalizeM2InboundQueryValue(
  value: QueryPanelValue,
  fallback: M2InboundQueryValue,
): M2InboundQueryValue {
  return {
    keyword: queryString(value.keyword),
    ownerKeyword: queryString(value.ownerKeyword) || fallback.ownerKeyword,
    documentTypeFilter: queryStringArray(value.documentTypeFilter) as InboundDocumentTypeFilter,
    statusFilter: queryStringArray(value.statusFilter) as StatusFilter,
    arrivalDate: queryRange(value.arrivalDate),
    createdAt: queryRange(value.createdAt, fallback.createdAt),
  };
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryStringArray(value: QueryPanelValue[string]) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

function queryRange(value: QueryPanelValue[string], fallback?: QueryPanelRangeValue): QueryPanelRangeValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return fallback ?? { from: "", to: "" };
  return {
    from: typeof value.from === "string" ? value.from : fallback?.from ?? "",
    to: typeof value.to === "string" ? value.to : fallback?.to ?? "",
  };
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}

function exampleText(value: string | number | null | undefined, fallback: string) {
  return value ? `例如 ${value}` : fallback;
}
