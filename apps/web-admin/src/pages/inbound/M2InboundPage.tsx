import * as React from "react";
import {
  Button,
  PageHeader,
  QueryPanel,
  buildQueryPanelSummaryItems,
  type QueryPanelField,
} from "@wms/ui";

import { useCurrentUserQuery } from "@/features/auth/auth-queries";
import { useMasterDataRowsQuery } from "@/features/master-data/master-data-queries";
import { useDualPersonPolicyQuery } from "@/features/validation-rules/dual-person-policy-queries";
import {
  useCreateReceivingOrderMutation,
  useInspectReceivingOrderMutation,
  usePutawayReceivingOrderMutation,
  useReleaseReceivingOrderMutation,
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
import { M2InboundPrintDialog } from "./M2InboundPrintDialog";
import { createAsnBatchNo } from "./m2-inbound-document-type";
import {
  dateToIso,
  dateTimeToIso,
  defaultM2InboundQueryValue,
  detailStageFromMode,
  filterOrders,
  inboundPageMeta,
  dualSignRequiredForPolicy,
  nextM2InboundSelectedId,
  normalizeM2InboundQueryValue,
  productTemperatureAttribute,
  queryValueFromUnknown,
  splitCodes,
  statusFilterOptions,
  temperatureControlFromProductAttribute,
  toInteger,
  totalExpectedQty,
  type M2InboundQueryValue,
  type M2InboundMode,
  type OwnerContext,
} from "./m2-inbound-page-helpers";
import { M2InboundOrderTable } from "./M2InboundOrderTable";
import { M2InboundDashboardPage } from "./M2InboundDashboardPage";

export type { M2InboundMode } from "./m2-inbound-page-helpers";

interface M2InboundPageProps {
  mode: M2InboundMode;
  currentOwner: OwnerContext;
  onBack: () => void;
}

/** 可读示例账号（placeholder / 第二收货员默认提示），勿用 UUID 样例。 */
const secondSignerExample = "00000000-0000-0000-0000-000000000102";
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
  samplingQty: "1",
  approvalNo: "",
  note: "",
};
const emptySignForm: SignFormState = {
  firstSignerId: "",
  secondSignerId: "",
  dualRequired: true,
  strategyNote: "",
  note: "",
};
const m2InboundCoreQueryFieldKeys = ["keyword", "ownerKeyword", "statusFilter"];

export function M2InboundPage({ mode, currentOwner }: M2InboundPageProps) {
  const currentUserQuery = useCurrentUserQuery(true);
  const currentUser = currentUserQuery.data;
  const m2InboundQueryFields: QueryPanelField[] = React.useMemo(() => [
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
      options: statusFilterOptions(mode),
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
  ], [mode]);
  const ordersQuery = useReceivingOrdersQuery();
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [selectionClearedByUser, setSelectionClearedByUser] = React.useState(false);
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
  const [showDashboard, setShowDashboard] = React.useState(false);
  const [detailOpen, setDetailOpen] = React.useState(false);
  const [printOpen, setPrintOpen] = React.useState(false);
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [putawayValidationError, setPutawayValidationError] = React.useState<string | null>(null);
  const [createForm, setCreateForm] = React.useState<CreateFormState>(emptyCreateForm);
  const [receiveForm, setReceiveForm] = React.useState<ReceiveFormState>({
    actualQty: "",
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
    deliveryQty: "",
    batchQty: "",
    secondReceiverId: secondSignerExample,
    note: "",
  });
  const [rejectForm, setRejectForm] = React.useState<RejectFormState>({
    reason: "",
  });
  const [inspectForm, setInspectForm] = React.useState<InspectFormState>(emptyInspectForm);
  const [signForm, setSignForm] = React.useState<SignFormState>(() =>
    createSignFormForCurrentUser(undefined),
  );
  const [putawayForm, setPutawayForm] = React.useState<PutawayFormState>({
    lpn: "",
    productCode: "",
    batchNo: "",
    qty: "0",
    locationId: "",
    locationCode: "",
    qualityStatus: "qualified",
    note: "",
  });

  const createMutation = useCreateReceivingOrderMutation();
  const receiveMutation = useReceiveReceivingOrderMutation();
  const releaseMutation = useReleaseReceivingOrderMutation();
  const rejectMutation = useRejectReceivingOrderMutation();
  const inspectMutation = useInspectReceivingOrderMutation();
  const signMutation = useSignReceivingOrderMutation();
  const putawayMutation = usePutawayReceivingOrderMutation();
  const locationsQuery = useMasterDataRowsQuery("m1-locations", mode === "putaway");
  const productsQuery = useMasterDataRowsQuery("m1-products", mode === "receiving");

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
    [appliedQuery, m2InboundQueryFields],
  );

  React.useEffect(() => {
    const orderIds = orders.map((order) => order.id);
    const nextSelectedId = nextM2InboundSelectedId(selectedId, orderIds, selectionClearedByUser);
    if (nextSelectedId !== selectedId) setSelectedId(nextSelectedId);
    setSelectedRowKeys((current) => {
      if (selectionClearedByUser) return current.length === 0 ? current : [];
      const validKeys = current.filter((key) => orderIds.includes(key));
      if (validKeys.length > 0) return sameStringArray(validKeys, current) ? current : validKeys;
      const nextKeys = nextSelectedId ? [nextSelectedId] : [];
      return sameStringArray(nextKeys, current) ? current : nextKeys;
    });
  }, [orders, selectedId, selectionClearedByUser]);

  React.useEffect(() => {
    const next = defaultM2InboundQueryValue(mode, currentOwner);
    setDraftQuery(next);
    setAppliedQuery(next);
    setLastEvent(null);
    setSelectionClearedByUser(false);
  }, [mode, currentOwner.ownerCode, currentOwner.ownerId]);

  const selectedFromList = ordersQuery.data?.find((order) => order.id === selectedId) ?? null;
  const detailQuery = useReceivingOrderQuery(selectedId);
  const order = detailQuery.data ?? selectedFromList;
  // optional chain must cover lines: `order?.lines[0]` still throws when order is null
  const line = order?.lines?.[0];
  const dualPolicyQuery = useDualPersonPolicyQuery(
    activeDialog === "inspect" && line?.product_id && order
      ? {
          productId: line.product_id,
          process: "入库",
          node: "验收",
          ownerId: currentOwner.ownerId,
          warehouseId: order.warehouse_id,
        }
      : null,
  );
  const dualSignRequiredByStrategy = dualSignRequiredForPolicy(dualPolicyQuery.data?.policy);
  const dualPolicyDescription = dualPolicyQuery.data?.policy === "dual_scan_with_approval"
    ? "M-VR：双人扫码 + 主管审批"
    : dualPolicyQuery.data?.policy === "dual_scan"
      ? "M-VR：双人扫码"
      : "M-VR：单人";
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
    firstSignerId: "当前用户 / 工号",
    secondSignerId: `例如 ${secondSignerExample}`,
    strategyNote: "例如 process=入库，node=验收，dual_scan",
  };
  const currentProduct = productsQuery.data?.find((product) => product.code === line?.product_code);
  const currentProductTemperatureAttribute = productTemperatureAttribute(
    currentProduct?.productFields?.storageCondition,
    line?.product_code,
  );
  const currentTemperatureControl = temperatureControlFromProductAttribute(currentProductTemperatureAttribute);
  const pending =
    createMutation.isPending ||
    releaseMutation.isPending ||
    receiveMutation.isPending ||
    rejectMutation.isPending ||
    inspectMutation.isPending ||
    signMutation.isPending ||
    putawayMutation.isPending || (mode === "putaway" && locationsQuery.isPending)
    || (activeDialog === "inspect" && dualPolicyQuery.isFetching);
  const error =
    createMutation.error ??
    releaseMutation.error ??
    receiveMutation.error ??
    rejectMutation.error ??
    inspectMutation.error ??
    signMutation.error ??
    putawayMutation.error ??
    locationsQuery.error ??
    ordersQuery.error ??
    detailQuery.error ??
    (activeDialog === "inspect" ? dualPolicyQuery.error : null);
  const errorMessage = putawayValidationError ?? (error ? inboundErrorMessage(error) : undefined);
  React.useEffect(() => {
    if (!order) return;
    const qty = String(totalExpectedQty(order));
    const firstLine = order.lines?.[0];
    const batchNo = firstLine?.batch_no?.trim() ?? "";
    setReceiveForm((value) => ({
      ...value,
      // 送货数量 / 实到数量默认带出订单预报数量，避免误导性 0
      actualQty: qty,
      deliveryQty: qty,
      // 批号字段默认用批号+预报数量，无批号时留空由用户录入（不要默认 0）
      batchQty: batchNo ? `${batchNo} × ${qty}` : "",
      shortageQty: "0",
      rejectedQty: "0",
      temperature: "",
      temperatureControl: value.temperatureControl,
    }));
    setRejectForm({ reason: "" });
    setInspectForm(emptyInspectForm);
    setSignForm(createSignFormForCurrentUser(currentUser, dualSignRequiredByStrategy));
    setPutawayForm((value) => ({
      ...value,
      productCode: firstLine?.product_code ?? "",
      batchNo,
      qty,
      locationId: "",
      locationCode: "",
    }));
  }, [order?.id, currentUser?.user_id, currentUser?.username, currentUser?.display_name, dualSignRequiredByStrategy]);

  React.useEffect(() => {
    setReceiveForm((value) =>
      value.temperatureControl === currentTemperatureControl
        ? value
        : { ...value, temperatureControl: currentTemperatureControl },
    );
  }, [order?.id, currentTemperatureControl]);

  async function refreshInbound(message = "入库列表已刷新") {
    await ordersQuery.refetch();
    if (selectedId) await detailQuery.refetch();
    setLastEvent(message);
  }

  function openRowDetail(id: string) {
    selectOrder(id);
    setDetailOpen(true);
  }

  function openRowPrint(id: string) {
    selectOrder(id);
    setPrintOpen(true);
  }

  function openRowDialog(id: string, dialog: InboundDialog) {
    selectOrder(id);
    if (dialog === "putaway") setPutawayValidationError(null);
    if (dialog === "inspect") {
      setInspectForm(emptyInspectForm);
      setSignForm(createSignFormForCurrentUser(currentUser));
    }
    setActiveDialog(dialog);
  }

  function selectOrder(id: string | null) {
    setSelectionClearedByUser(id === null);
    setSelectedId(id);
    setSelectedRowKeys(id ? [id] : []);
  }

  function selectOrderKeys(keys: string[]) {
    const visibleOrderIds = new Set(orders.map((order) => order.id));
    const clearedVisibleSelection = keys.length === 0 && selectedRowKeys.some((key) => visibleOrderIds.has(key));
    setSelectionClearedByUser(clearedVisibleSelection);
    setSelectedRowKeys(keys);
    setSelectedId(keys.at(-1) ?? null);
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
    await ordersQuery.refetch();
    selectOrder(created.id);
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
        details: {
          temperature_control_method: coldChain ? receiveForm.temperatureControl.trim() || currentTemperatureControl : null,
          vehicle_no: receiveForm.vehicleNo.trim() || null,
          origin: receiveForm.origin.trim() || null,
          departure_at: coldChain ? dateTimeToIso(receiveForm.departureTime) : null,
          arrival_at: coldChain ? dateTimeToIso(receiveForm.arrivalTime) : null,
          storage_at: dateTimeToIso(receiveForm.storageTime),
          transport_mode: receiveForm.transportMode.trim() || null,
          carrier: receiveForm.carrier.trim() || null,
          contact_name: receiveForm.contactName.trim() || null,
          contact_phone: receiveForm.contactPhone.trim() || null,
          contact_id_no: receiveForm.contactIdNo.trim() || null,
          seal_checked: receiveForm.sealChecked.trim() || null,
          filing_checked: receiveForm.filingChecked.trim() || null,
        },
      },
    });
    setActiveDialog(null);
    setLastEvent(`${order.receipt_no} 收货已提交`);
  }

  async function releaseOrder(id: string) {
    const released = await releaseMutation.mutateAsync(id);
    setLastEvent(`${released.receipt_no} 已放行`);
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
    if (!order || !currentUser) return;
    // 策略要求时强制 dual_required，作业员不可关闭
    const dualRequired = dualSignRequiredByStrategy || signForm.dualRequired;
    const currentUserId = currentUser.user_id;
    // 双人验收必须分两次独立认证：当前登录用户作为本步签字主体，禁止代填他人 ID。
    if (order.status === "awaiting_second_sign") {
      // 第二步只声明当前用户为第二人；第一人 ID 由服务端从既有签名记录读取。
      await signMutation.mutateAsync({
        id: order.id,
        request: {
          first_signer_id: "00000000-0000-0000-0000-000000000000",
          second_signer_id: currentUserId,
          dual_required: true,
        },
      });
      setActiveDialog(null);
      setLastEvent(`${order.receipt_no} 第二人签字已完成`);
      return;
    }
    if (order.status === "inspecting" || order.status === "receiving" || order.status === "received") {
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
          appearance_check: inspectForm.appearanceCheck.trim() || null,
          package_check: inspectForm.packageCheck.trim() || null,
          instruction_check: inspectForm.instructionCheck.trim() || null,
          label_check: inspectForm.labelCheck.trim() || null,
          sampling_qty: toInteger(inspectForm.samplingQty || "0"),
          approval_no: inspectForm.approvalNo.trim() || null,
        },
      });
    }
    await signMutation.mutateAsync({
      id: order.id,
      request: {
        first_signer_id: currentUserId,
        second_signer_id: null,
        dual_required: dualRequired,
      },
    });
    setActiveDialog(null);
    setLastEvent(
      dualRequired
        ? `${order.receipt_no} 第一人已签字，待第二人独立登录签字`
        : `${order.receipt_no} 验收已提交`,
    );
  }

  async function submitPutaway(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!order) return;
    const qty = toInteger(putawayForm.qty);
    if (qty <= 0) {
      setPutawayValidationError("上架数量必须大于 0");
      return;
    }
    const locationCode = putawayForm.locationCode.trim();
    const location = locationsQuery.data?.find(
      (row) => row.code === locationCode && row.locationFields?.warehouseId === order.warehouse_id,
    ) ?? (putawayForm.locationId && locationCode ? { id: putawayForm.locationId } : null);
    if (!location) {
      setPutawayValidationError(`库位 ${locationCode || "-"} 不存在，或不属于当前入库仓库`);
      return;
    }
    setPutawayValidationError(null);
    await putawayMutation.mutateAsync({
      id: order.id,
      request: {
        product_code: putawayForm.productCode.trim(),
        batch_no: putawayForm.batchNo.trim(),
        qty,
        location_id: location.id,
        location_code: locationCode,
        quality_status: putawayForm.qualityStatus.trim(),
        lpn_code: putawayForm.lpn.trim() || null,
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

  if (showDashboard && mode === "receiving") {
    return <M2InboundDashboardPage currentOwner={currentOwner} onBack={() => setShowDashboard(false)} />;
  }

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
              {mode === "receiving" && (
                <Button variant="outline" onClick={() => setShowDashboard(true)}>进度看板</Button>
              )}
            </div>
          }
        />

        <QueryPanel
          fields={m2InboundQueryFields}
          defaultVisibleFieldKeys={m2InboundCoreQueryFieldKeys}
          value={draftQuery}
          onValueChange={(next) => setDraftQuery(normalizeM2InboundQueryValue(next, defaultQuery, mode))}
          onQuery={() => {
            setAppliedQuery(normalizeM2InboundQueryValue(draftQuery, defaultQuery, mode));
            setSelectionClearedByUser(false);
            void refreshInbound("入库列表已查询");
          }}
          onReset={() => {
            const next = defaultM2InboundQueryValue(mode, currentOwner);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectionClearedByUser(false);
            setSelectedRowKeys([]);
            setSelectedId(null);
          }}
        />

        {errorMessage && (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {errorMessage}
          </div>
        )}

        <M2InboundOrderTable
          mode={mode}
          currentOwner={currentOwner}
          orders={orders}
          exportFileBaseName={pageMeta.title}
          selectedId={selectedId}
          selectedRowKeys={selectedRowKeys}
          isPending={ordersQuery.isPending}
          onSelectOrder={selectOrder}
          onSelectOrderKeys={selectOrderKeys}
          onOpenDetail={openRowDetail}
          onOpenPrint={openRowPrint}
          onOpenDialog={openRowDialog}
          onRelease={(id) => void releaseOrder(id)}
          refreshAction={tableRefreshAction}
          createAction={tableCreateAction}
          queryState={appliedQuery}
          querySummaryItems={m2QuerySummaryItems}
          onApplyQueryState={(queryState) => {
            const next = normalizeM2InboundQueryValue(queryValueFromUnknown(queryState), defaultQuery, mode);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectionClearedByUser(false);
            setSelectedRowKeys([]);
            setSelectedId(null);
          }}
          onClearQueryState={() => {
            const next = defaultM2InboundQueryValue(mode, currentOwner);
            setDraftQuery(next);
            setAppliedQuery(next);
            setSelectionClearedByUser(false);
            setSelectedRowKeys([]);
            setSelectedId(null);
          }}
        />

        <M2InboundDialogs
          activeDialog={activeDialog}
          orderId={order?.id ?? null}
          orderReceiptNo={order?.receipt_no ?? null}
          hasOrder={Boolean(order)}
          pending={pending}
          errorMessage={errorMessage}
          productTemperatureAttribute={currentProductTemperatureAttribute}
          derivedTemperatureControl={currentTemperatureControl}
          createForm={createForm}
          receiveForm={receiveForm}
          rejectForm={rejectForm}
          inspectForm={inspectForm}
          inspectExamples={inspectExamples}
          signForm={signForm}
          secondSignature={order?.status === "awaiting_second_sign"}
          dualSignRequiredByStrategy={dualSignRequiredByStrategy}
          dualPolicyDescription={dualPolicyDescription}
          putawayForm={putawayForm}
          setActiveDialog={setActiveDialog}
          setCreateForm={setCreateForm}
          setReceiveForm={setReceiveForm}
          setRejectForm={setRejectForm}
          setInspectForm={setInspectForm}
          setSignForm={setSignForm}
          setPutawayForm={setPutawayForm}
          clearPutawayValidationError={() => setPutawayValidationError(null)}
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
        <M2InboundPrintDialog
          currentOwner={currentOwner}
          mode={mode}
          onOpenChange={setPrintOpen}
          onPrinted={(receiptNo) => setLastEvent(`${receiptNo} 打印记录已写入`)}
          open={printOpen}
          order={order}
        />
    </section>
  );
}

function sameStringArray(left: string[], right: string[]) {
  return left.length === right.length && left.every((item, index) => item === right[index]);
}

function createSignFormForCurrentUser(
  user: { username?: string; display_name?: string } | null | undefined,
  dualSignRequired = false,
): SignFormState {
  const account = user?.username?.trim() || user?.display_name?.trim() || "";
  return {
    ...emptySignForm,
    firstSignerId: account,
    dualRequired: dualSignRequired,
  };
}

/**
 * 签字字段展示账号/工号；提交时若匹配当前登录用户则映射为 user_id（API 契约为 id）。
 * 未匹配时仅允许手填 UUID；账号解析需等待受控人员目录接口。
 */
function resolveSignerIdForSubmit(
  input: string,
  user: { user_id: string; username?: string; display_name?: string } | null | undefined,
  options?: { allowCurrentUser?: boolean },
) {
  if (!input) return input;
  const allowCurrentUser = options?.allowCurrentUser !== false;
  if (
    allowCurrentUser &&
    user &&
    (input === user.user_id || input === user.username || input === user.display_name)
  ) {
    return user.user_id;
  }
  return input;
}

function isUuid(value: string) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
}

function exampleText(value: string | number | null | undefined, fallback: string) {
  return value ? `例如 ${value}` : fallback;
}

function inboundErrorMessage(error: { code?: string; message: string }) {
  if (
    error.code === "DEV_MOCK_NOT_FOUND" ||
    error.code === "INBOUND_ORDER_NOT_FOUND" ||
    /Dev mock route not found|Receiving order not found/i.test(error.message)
  ) {
    return "未找到匹配的入库数据，请调整查询条件或刷新后重试";
  }
  return error.message;
}
