/**
 * M2InboundDocumentsPage — 入库资料录入
 *
 * 层级：Layer 3 页面
 * 关联故事：US-DI-001
 * Wave：Wave 6
 * 业务约束：列表按实际收货时间查询；药检单按批号录入，上游随货同行单可关联同供应商多个 ASN。
 *
 * @example
 *   <M2InboundDocumentsPage />
 */

import * as React from "react";
import {
  Button,
  Card,
  CardContent,
  Checkbox,
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
  QueryPanel,
  StatusBadge,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  buildQueryPanelSummaryItems,
  cn,
  type DataGridColumn,
  type DataGridQuerySummaryItem,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelRangeValue,
  type QueryPanelValue,
} from "@wms/ui";
import { FileUp } from "lucide-react";

import {
  createDrugInspectionImagePreview,
  type InboundDocumentEntry,
  uploadDrugInspectionAttachment,
  useInboundDocumentsQuery,
  useSaveDrugInspectionMutation,
  useSaveUpstreamDeliveryMutation,
} from "@/features/drug-inspection/document-queries";
import {
  defaultInboundDocumentQuery,
  filterInboundDocumentRows,
  toWmsBusinessDate,
  validateDrugInspectionFile,
  validateUpstreamDeliveryFiles,
  type InboundDocumentEntryRow,
} from "./inbound-document-entry-model";
import {
  BUTTON_REFRESH,
  COLUMN_BATCH_NO,
  COLUMN_CREATED_AT,
  FIELD_KEYWORD,
  LOADING_SAVING,
} from "@/lib/ui-strings";

export const m2InboundDocumentsQueryFields: QueryPanelField[] = [
  { key: "keyword", label: FIELD_KEYWORD, type: "text", placeholder: "ASN / 采购订单 / 供应商 / 商品 / 批号" },
  { key: "actualReceivedAt", label: "实际收货时间", type: "dateRange" },
  { key: "supplierKeyword", label: "供应商", type: "text", placeholder: "供应商名称 / ID" },
];
export const m2InboundDocumentsCoreQueryFieldKeys = ["keyword", "actualReceivedAt"];

type EntryTab = "drug" | "upstream";
type Notice = { kind: "success" | "error"; text: string } | null;

export function M2InboundDocumentsPage() {
  const defaultQuery = React.useMemo(defaultInboundDocumentQuery, []);
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => toPanelValue(defaultQuery));
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => toPanelValue(defaultQuery));
  const [missingDrugInspection, setMissingDrugInspection] = React.useState(false);
  const [missingUpstreamDelivery, setMissingUpstreamDelivery] = React.useState(false);
  const [activeRowId, setActiveRowId] = React.useState<string | null>(null);
  const [activeTab, setActiveTab] = React.useState<EntryTab>("drug");
  const [drugSource, setDrugSource] = React.useState<"upload" | "reuse">("upload");
  const [processingMode, setProcessingMode] = React.useState<
    "none" | "color_enhance" | "black_white_enhance"
  >("none");
  const [selectedBatch, setSelectedBatch] = React.useState("");
  const [selectedAsnIds, setSelectedAsnIds] = React.useState<string[]>([]);
  const [candidateFrom, setCandidateFrom] = React.useState("");
  const [candidateTo, setCandidateTo] = React.useState("");
  const [files, setFiles] = React.useState<File[]>([]);
  const [reportNo, setReportNo] = React.useState("");
  const [qualified, setQualified] = React.useState(true);
  const [reason, setReason] = React.useState("");
  const [notice, setNotice] = React.useState<Notice>(null);
  const [previewAttachmentId, setPreviewAttachmentId] = React.useState<string>();
  const [originalPreviewUrl, setOriginalPreviewUrl] = React.useState("");
  const [processedPreviewUrl, setProcessedPreviewUrl] = React.useState("");
  const [previewDimensions, setPreviewDimensions] = React.useState("");
  const [previewStatus, setPreviewStatus] = React.useState<"idle" | "uploading" | "ready" | "error">("idle");
  const previewRequestRef = React.useRef(0);
  const processingModeRef = React.useRef(processingMode);
  const applied = toModelQuery(
    appliedQuery,
    missingDrugInspection,
    missingUpstreamDelivery,
  );
  const documentsQuery = useInboundDocumentsQuery({
    receivedFrom: applied.receivedFrom,
    receivedTo: applied.receivedTo,
    missingDrugInspection: false,
    missingUpstreamDelivery: false,
  });
  const saveDrugInspection = useSaveDrugInspectionMutation();
  const saveUpstreamDelivery = useSaveUpstreamDeliveryMutation();
  const rows = React.useMemo(
    () => (documentsQuery.data ?? []).map(toRow),
    [documentsQuery.data],
  );
  const activeRow = rows.find((row) => row.id === activeRowId) ?? null;
  const filteredRows = React.useMemo(
    () => filterInboundDocumentRows(rows, applied),
    [rows, applied.keyword, applied.receivedFrom, applied.receivedTo, applied.missingDrugInspection, applied.missingUpstreamDelivery],
  );
  const drugMissingCount = React.useMemo(
    () => filterInboundDocumentRows(rows, { ...applied, missingDrugInspection: true, missingUpstreamDelivery: false }).length,
    [rows, applied.keyword, applied.receivedFrom, applied.receivedTo],
  );
  const upstreamMissingCount = React.useMemo(
    () => filterInboundDocumentRows(rows, { ...applied, missingDrugInspection: false, missingUpstreamDelivery: true }).length,
    [rows, applied.keyword, applied.receivedFrom, applied.receivedTo],
  );
  const candidates = React.useMemo(() => {
    if (!activeRow) return [];
    return rows.filter((row) => {
      const receivedDate = row.actualReceivedAt ? toWmsBusinessDate(row.actualReceivedAt) : "";
      return row.supplierId === activeRow.supplierId
        && Boolean(receivedDate)
        && (!candidateFrom || receivedDate >= candidateFrom)
        && (!candidateTo || receivedDate <= candidateTo);
    });
  }, [rows, activeRow, candidateFrom, candidateTo]);
  React.useEffect(
    () => () => {
      if (originalPreviewUrl) URL.revokeObjectURL(originalPreviewUrl);
    },
    [originalPreviewUrl],
  );
  React.useEffect(
    () => () => {
      if (processedPreviewUrl) URL.revokeObjectURL(processedPreviewUrl);
    },
    [processedPreviewUrl],
  );

  const columns = React.useMemo<DataGridColumn<InboundDocumentEntryRow>[]>(() => [
    textColumn("receiptNo", "ASN", 180, true),
    textColumn("purchaseOrderNo", "采购订单", 180, true),
    {
      key: "actualReceivedAt",
      header: "实际收货时间",
      width: 180,
      sortable: true,
      sortValue: (row) => row.actualReceivedAt ?? "",
      filterValue: (row) => row.actualReceivedAt,
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.actualReceivedAt),
    },
    textColumn("supplierName", "供应商", 180),
    {
      key: "productBatch",
      header: "商品 / 批号",
      width: 230,
      filterValue: (row) => `${row.productCode} ${row.batchNos.join(" ")}`,
      render: (row) => <div><div className="font-medium">{row.productCode}</div><div className="text-xs text-muted-foreground">{row.batchNos.join("、") || "待批号"}</div></div>,
    },
    {
      key: "drugInspectionStatus",
      header: "药检单",
      width: 150,
      filterValue: (row) => drugStatusLabel(row.drugInspectionStatus),
      render: (row) => <StatusBadge size="sm" status={drugStatusVariant(row.drugInspectionStatus)} label={`${drugStatusLabel(row.drugInspectionStatus)}${row.drugInspectionVersion ? ` v${row.drugInspectionVersion}` : ""}`} />,
    },
    {
      key: "upstreamDeliveryStatus",
      header: "上游随货同行单",
      width: 180,
      filterValue: (row) => upstreamStatusLabel(row.upstreamDeliveryStatus),
      render: (row) => <StatusBadge size="sm" status={row.upstreamDeliveryStatus === "uploaded" ? "completed" : "pending"} label={`${upstreamStatusLabel(row.upstreamDeliveryStatus)}${row.upstreamVersion ? ` v${row.upstreamVersion}` : ""}`} />,
    },
    {
      key: "overallStatus",
      header: "资料状态",
      width: 130,
      filterValue: overallStatusLabel,
      render: (row) => <StatusBadge size="sm" status={overallStatusVariant(row)} label={overallStatusLabel(row)} />,
    },
    {
      key: "createdAt",
      header: COLUMN_CREATED_AT,
      width: 180,
      sortable: true,
      sortValue: (row) => row.createdAt,
      filterValue: (row) => row.createdAt,
      filter: { type: "dateRange" },
      render: (row) => formatDateTime(row.createdAt),
    },
    {
      key: "actions",
      header: "操作",
      width: 130,
      render: (row) => <Button type="button" size="sm" variant="outline" onClick={(event) => { event.stopPropagation(); openEntry(row); }}><FileUp className="size-4" aria-hidden />录入资料</Button>,
    },
  ], []);

  const querySummaryItems = React.useMemo<DataGridQuerySummaryItem[]>(() => [
    ...buildQueryPanelSummaryItems(m2InboundDocumentsQueryFields, appliedQuery),
    ...(missingDrugInspection ? [{ key: "missingDrugInspection", label: "快捷筛选", value: "药检单不齐", text: "快捷筛选：药检单不齐" }] : []),
    ...(missingUpstreamDelivery ? [{ key: "missingUpstreamDelivery", label: "快捷筛选", value: "上游随货同行单不齐", text: "快捷筛选：上游随货同行单不齐" }] : []),
  ], [appliedQuery, missingDrugInspection, missingUpstreamDelivery]);
  const refreshAction: DataGridRefreshAction = {
    label: BUTTON_REFRESH,
    description: "刷新入库资料清单",
    disabled: documentsQuery.isFetching,
    onClick: () => void documentsQuery.refetch(),
  };

  function resetQuery() {
    const next = toPanelValue(defaultQuery);
    setDraftQuery(next);
    setAppliedQuery(next);
    setMissingDrugInspection(false);
    setMissingUpstreamDelivery(false);
  }
  function toggleQuickFilter(key: string) {
    if (key === "drug") setMissingDrugInspection((current) => !current);
    if (key === "upstream") setMissingUpstreamDelivery((current) => !current);
  }
  function openEntry(row: InboundDocumentEntryRow) {
    const date = row.actualReceivedAt ? toWmsBusinessDate(row.actualReceivedAt) : "";
    setActiveRowId(row.id);
    setActiveTab("drug");
    setDrugSource("upload");
    setProcessingMode("none");
    processingModeRef.current = "none";
    setSelectedBatch(row.batchNos[0] ?? "");
    setSelectedAsnIds([row.id]);
    setCandidateFrom(date);
    setCandidateTo(date);
    setFiles([]);
    setReportNo("");
    setQualified(true);
    setReason("");
    setNotice(null);
    resetImagePreview();
  }
  function closeEntry() {
    setActiveRowId(null);
    setFiles([]);
    setReason("");
    resetImagePreview();
  }
  function resetImagePreview() {
    previewRequestRef.current += 1;
    setPreviewAttachmentId(undefined);
    setOriginalPreviewUrl("");
    setProcessedPreviewUrl("");
    setPreviewDimensions("");
    setPreviewStatus("idle");
  }
  async function selectDrugInspectionFile(file: File | undefined) {
    resetImagePreview();
    setFiles(file ? [file] : []);
    if (!file || !activeRow) return;
    const validationError = validateDrugInspectionFile(file);
    if (validationError) {
      setNotice({ kind: "error", text: validationError });
      setPreviewStatus("error");
      return;
    }
    setNotice(null);
    if (!["image/jpeg", "image/png"].includes(file.type)) {
      setProcessingMode("none");
      processingModeRef.current = "none";
      return;
    }
    setOriginalPreviewUrl(URL.createObjectURL(file));
    setPreviewStatus("uploading");
    const requestId = previewRequestRef.current;
    try {
      const attachmentId = await uploadDrugInspectionAttachment({
        file,
        entityId: activeRow.id,
        entityType: "drug_inspection_original",
      });
      if (requestId !== previewRequestRef.current) return;
      setPreviewAttachmentId(attachmentId);
      await refreshProcessedPreview(attachmentId, processingModeRef.current, requestId);
    } catch (error) {
      if (requestId !== previewRequestRef.current) return;
      setPreviewStatus("error");
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "生成药检图片预览失败" });
    }
  }
  async function refreshProcessedPreview(
    attachmentId: string,
    mode: "none" | "color_enhance" | "black_white_enhance",
    requestId = ++previewRequestRef.current,
  ) {
    setPreviewStatus("uploading");
    try {
      const preview = await createDrugInspectionImagePreview({
        attachmentId,
        processingMode: mode,
      });
      if (requestId !== previewRequestRef.current) return;
      const decoded = globalThis.atob(preview.data_base64);
      const bytes = Uint8Array.from(decoded, (character) => character.charCodeAt(0));
      setProcessedPreviewUrl(URL.createObjectURL(new Blob(
        [bytes],
        { type: preview.content_type },
      )));
      setPreviewDimensions(`${preview.width} × ${preview.height}`);
      setPreviewStatus("ready");
    } catch (error) {
      if (requestId !== previewRequestRef.current) return;
      setPreviewStatus("error");
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "生成药检图片预览失败" });
    }
  }
  function changeProcessingMode(value: string) {
    const mode = value as "none" | "color_enhance" | "black_white_enhance";
    setProcessingMode(mode);
    processingModeRef.current = mode;
    if (previewAttachmentId) void refreshProcessedPreview(previewAttachmentId, mode);
  }
  function toggleCandidate(id: string, checked: boolean) {
    setSelectedAsnIds((current) => checked
      ? Array.from(new Set([...current, id]))
      : current.filter((item) => item !== id));
  }
  async function submitDrugInspection() {
    if (!activeRow) return;
    if (!selectedBatch) { setNotice({ kind: "error", text: "实际收货并生成批号后才能录入药检单" }); return; }
    if (drugSource === "upload") {
      const error = validateDrugInspectionFile(files[0] ?? null);
      if (error) { setNotice({ kind: "error", text: error }); return; }
      if (!reportNo.trim()) { setNotice({ kind: "error", text: "请输入报告编号" }); return; }
    }
    try {
      await saveDrugInspection.mutateAsync({
        row: toApiRow(activeRow),
        batchNo: selectedBatch,
        reportNo: reportNo.trim(),
        file: files[0],
        processingMode: processingMode as "none" | "color_enhance" | "black_white_enhance",
        qualified,
        source: drugSource,
        modificationReason: reason.trim() || undefined,
        attachmentId: previewAttachmentId,
      });
      setNotice({ kind: "success", text: `${selectedBatch} 药检单已${drugSource === "reuse" ? "复用" : "上传并提交审核"}` });
      setFiles([]);
      setReportNo("");
      setReason("");
      resetImagePreview();
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "保存药检单失败" });
    }
  }
  async function submitUpstreamDelivery() {
    if (!activeRow) return;
    const error = validateUpstreamDeliveryFiles(files);
    if (error) { setNotice({ kind: "error", text: error }); return; }
    if (selectedAsnIds.length === 0) { setNotice({ kind: "error", text: "至少关联一个 ASN" }); return; }
    if (activeRow.upstreamVersion > 0 && !reason.trim()) {
      setNotice({ kind: "error", text: "重新上传上游随货同行单必须填写修改原因" });
      return;
    }
    try {
      await saveUpstreamDelivery.mutateAsync({
        row: toApiRow(activeRow),
        asnIds: selectedAsnIds,
        files,
        modificationReason: reason.trim() || undefined,
      });
      setNotice({ kind: "success", text: `上游随货同行单已关联 ${selectedAsnIds.length} 个 ASN` });
      setFiles([]);
      setReason("");
    } catch (error) {
      setNotice({ kind: "error", text: error instanceof Error ? error.message : "上传上游随货同行单失败" });
    }
  }

  return <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
    <PageHeader title="入库资料录入" />
    {notice && <div role={notice.kind === "error" ? "alert" : "status"} className={cn("rounded-md border px-3 py-2 text-sm", notice.kind === "error" ? "border-destructive/30 bg-destructive/10 text-destructive" : "border-wms-success/30 bg-wms-success/10 text-wms-success")}>{notice.text}</div>}
    <QueryPanel
      fields={m2InboundDocumentsQueryFields}
      defaultVisibleFieldKeys={m2InboundDocumentsCoreQueryFieldKeys}
      value={draftQuery}
      onValueChange={setDraftQuery}
      quickFilters={[
        { key: "drug", label: `药检单不齐 ${drugMissingCount}`, active: missingDrugInspection },
        { key: "upstream", label: `上游随货同行单不齐 ${upstreamMissingCount}`, active: missingUpstreamDelivery },
      ]}
      quickFiltersAriaLabel="资料快捷筛选"
      onQuickFilterClick={toggleQuickFilter}
      onQuery={() => setAppliedQuery(draftQuery)}
      onReset={resetQuery}
    />
    <Card><CardContent className="p-5"><DataGrid
      storageKey="m2.inbound-documents"
      columns={columns}
      data={filteredRows}
      rowKey={(row) => row.id}
      caption={documentsQuery.isPending ? "加载入库资料..." : `共 ${filteredRows.length} 个 ASN`}
      emptyTitle={documentsQuery.isError ? "读取入库资料失败" : "当前条件下没有 ASN"}
      emptyDescription={documentsQuery.isError ? documentsQuery.error.message : "调整实际收货日期或快捷筛选后重试"}
      tableClassName="min-w-[1680px]"
      exportFileBaseName="入库资料录入"
      refreshAction={refreshAction}
      queryState={appliedQuery}
      querySummaryItems={querySummaryItems}
      onApplyQueryState={(value) => { const next = normalizePanelValue(value); setDraftQuery(next); setAppliedQuery(next); }}
      onClearQueryState={resetQuery}
    /></CardContent></Card>

    {activeRow && <Dialog open onOpenChange={(open) => !open && closeEntry()}><DialogContent className="max-h-[92vh] overflow-y-auto sm:max-w-4xl">
      <DialogHeader><DialogTitle>录入资料 · {activeRow.receiptNo}</DialogTitle><DialogDescription>采购订单 {activeRow.purchaseOrderNo} · {activeRow.supplierName} · 所有上传、复用和版本修改均写入审计。</DialogDescription></DialogHeader>
      {notice && <div role={notice.kind === "error" ? "alert" : "status"} className={cn("rounded-md border px-3 py-2 text-sm", notice.kind === "error" ? "border-destructive/30 bg-destructive/10 text-destructive" : "border-wms-success/30 bg-wms-success/10 text-wms-success")}>{notice.text}</div>}
      <Tabs value={activeTab} onValueChange={(value) => { setActiveTab(value as EntryTab); setFiles([]); setReason(""); setNotice(null); resetImagePreview(); }}>
        <TabsList><TabsTrigger value="drug">药检单</TabsTrigger><TabsTrigger value="upstream">上游随货同行单</TabsTrigger></TabsList>
        <TabsContent value="drug" className="mt-4 grid gap-4">
          <div className="grid gap-4 md:grid-cols-2">
            <Field label={COLUMN_BATCH_NO}><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={selectedBatch} onChange={(event) => setSelectedBatch(event.target.value)}>{activeRow.batchNos.length ? activeRow.batchNos.map((batchNo) => <option key={batchNo}>{batchNo}</option>) : <option value="">待批号</option>}</select></Field>
            <Field label="当前版本"><Input readOnly value={activeRow.drugInspectionVersion ? `v${activeRow.drugInspectionVersion}` : "未录入"} /></Field>
            <Field label="录入方式"><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={drugSource} onChange={(event) => { setDrugSource(event.target.value as "upload" | "reuse"); setFiles([]); resetImagePreview(); }}><option value="upload">上传新文件</option><option value="reuse">复用已有药检单</option></select></Field>
            {drugSource === "upload" && <Field label="图片处理方式"><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={processingMode} onChange={(event) => changeProcessingMode(event.target.value)}><option value="none">不处理</option><option value="color_enhance">原色增强</option><option value="black_white_enhance">黑白增强</option></select></Field>}
            {drugSource === "upload" && <Field label="报告编号"><Input required value={reportNo} onChange={(event) => setReportNo(event.target.value)} placeholder="请输入药检报告编号" /></Field>}
            {drugSource === "upload" && <Field label="检验结论"><select className="h-10 rounded-md border border-input bg-background px-3 text-sm" value={qualified ? "qualified" : "unqualified"} onChange={(event) => setQualified(event.target.value === "qualified")}><option value="qualified">合格</option><option value="unqualified">不合格</option></select></Field>}
          </div>
          {drugSource === "upload"
            ? <Field label="药检单文件"><Input type="file" accept=".pdf,.jpg,.jpeg,.png" onChange={(event) => void selectDrugInspectionFile(event.target.files?.[0])} /><span className="text-xs text-muted-foreground">PDF 不超过 50MB；JPG/PNG 不超过 5MB。图片上传后通过真实 H-FILE 和处理服务生成预览，权威原件不覆盖。</span></Field>
            : <Field label="可复用药检单"><Input readOnly value={selectedBatch ? `${activeRow.productCode} / ${selectedBatch} / 当前已确认版本` : "请先选择批号"} /></Field>}
          {drugSource === "upload" && originalPreviewUrl && (
            <div className="grid gap-3 rounded-md border bg-muted/20 p-3 md:grid-cols-2" data-testid="drug-inspection-image-preview">
              <ImagePreview title="权威原图" src={originalPreviewUrl} />
              <ImagePreview
                title={`处理后预览${previewDimensions ? ` · ${previewDimensions}` : ""}`}
                src={processedPreviewUrl}
                pending={previewStatus === "uploading"}
                error={previewStatus === "error"}
              />
            </div>
          )}
          {drugSource === "upload" && (
            <Field label="修改原因">
              <Input
                value={reason}
                onChange={(event) => setReason(event.target.value)}
                placeholder="当前批号已有版本时必填；新批号可留空"
              />
            </Field>
          )}
          <VersionNote row={activeRow} kind="drug" />
          <DialogFooter><DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose><Button type="button" disabled={!selectedBatch || saveDrugInspection.isPending || previewStatus === "uploading" || previewStatus === "error"} onClick={() => void submitDrugInspection()}><FileUp className="size-4" aria-hidden />{saveDrugInspection.isPending ? LOADING_SAVING : previewStatus === "uploading" ? "生成预览中..." : "保存药检单"}</Button></DialogFooter>
        </TabsContent>
        <TabsContent value="upstream" className="mt-4 grid gap-4">
          <div className="grid gap-4 md:grid-cols-3">
            <Field label="供应商"><Input readOnly value={activeRow.supplierName} /></Field>
            <Field label="收货日期开始"><Input type="date" value={candidateFrom} onChange={(event) => setCandidateFrom(event.target.value)} /></Field>
            <Field label="收货日期结束"><Input type="date" value={candidateTo} onChange={(event) => setCandidateTo(event.target.value)} /></Field>
          </div>
          <div className="rounded-md border">
            <div className="border-b bg-muted/40 px-3 py-2 text-sm font-medium">同供应商可关联 ASN（当前 ASN 默认选中）</div>
            <div className="max-h-44 overflow-y-auto p-3">
              {candidates.map((row) => <label key={row.id} className="flex items-center gap-3 border-b py-2 last:border-0"><Checkbox checked={selectedAsnIds.includes(row.id)} onCheckedChange={(checked) => toggleCandidate(row.id, checked === true)} /><span className="font-mono text-sm">{row.receiptNo}</span><span className="text-sm text-muted-foreground">{row.actualReceivedAt ? toWmsBusinessDate(row.actualReceivedAt) : "未收货"} · {upstreamStatusLabel(row.upstreamDeliveryStatus)}</span></label>)}
              {candidates.length === 0 && <p className="text-sm text-muted-foreground">当前日期范围没有可关联 ASN。</p>}
            </div>
          </div>
          <Field label="上游随货同行单"><Input type="file" accept=".pdf,.jpg,.jpeg" multiple onChange={(event) => setFiles(Array.from(event.target.files ?? []))} /><span className="text-xs text-muted-foreground">上传一个 PDF 或多张 JPG，每个文件不超过 5MB；上传即完成录入。</span></Field>
          {activeRow.upstreamVersion > 0 && <Field label="修改原因"><Input required value={reason} onChange={(event) => setReason(event.target.value)} placeholder="重新上传必须填写" /></Field>}
          <VersionNote row={activeRow} kind="upstream" />
          <DialogFooter><DialogClose asChild><Button type="button" variant="outline">取消</Button></DialogClose><Button type="button" disabled={saveUpstreamDelivery.isPending} onClick={() => void submitUpstreamDelivery()}><FileUp className="size-4" aria-hidden />{saveUpstreamDelivery.isPending ? "上传中..." : "上传并完成录入"}</Button></DialogFooter>
        </TabsContent>
      </Tabs>
    </DialogContent></Dialog>}
  </section>;
}

function toPanelValue(query: ReturnType<typeof defaultInboundDocumentQuery>): QueryPanelValue {
  return { keyword: query.keyword, actualReceivedAt: { from: query.receivedFrom, to: query.receivedTo }, supplierKeyword: "" };
}
function toRow(value: InboundDocumentEntry): InboundDocumentEntryRow {
  return {
    id: value.asn_id,
    receiptNo: value.receipt_no,
    purchaseOrderNo: value.purchase_order_no,
    ownerId: value.owner_id,
    supplierId: value.supplier_id,
    supplierName: value.supplier_name,
    productId: value.product_id,
    productCode: value.product_code,
    productName: value.product_name,
    batchNos: value.batch_nos,
    actualReceivedAt: value.actual_received_at ?? null,
    drugInspectionStatus: value.drug_inspection_status as InboundDocumentEntryRow["drugInspectionStatus"],
    drugInspectionVersion: value.drug_inspection_version,
    upstreamDeliveryStatus: value.upstream_delivery_status as InboundDocumentEntryRow["upstreamDeliveryStatus"],
    upstreamVersion: value.upstream_version,
    upstreamDocumentId: value.upstream_document_id ?? undefined,
    createdAt: value.created_at,
  };
}
function toApiRow(value: InboundDocumentEntryRow): InboundDocumentEntry {
  return {
    asn_id: value.id,
    receipt_no: value.receiptNo,
    purchase_order_no: value.purchaseOrderNo,
    owner_id: value.ownerId,
    supplier_id: value.supplierId,
    supplier_name: value.supplierName,
    product_id: value.productId,
    product_code: value.productCode,
    product_name: value.productName,
    batch_nos: value.batchNos,
    actual_received_at: value.actualReceivedAt ?? undefined,
    drug_inspection_status: value.drugInspectionStatus,
    drug_inspection_version: value.drugInspectionVersion,
    upstream_delivery_status: value.upstreamDeliveryStatus,
    upstream_version: value.upstreamVersion,
    upstream_document_id: value.upstreamDocumentId,
    created_at: value.createdAt,
  };
}
function normalizePanelValue(value: unknown): QueryPanelValue {
  const row = value && typeof value === "object" ? value as QueryPanelValue : {};
  const range = row.actualReceivedAt && typeof row.actualReceivedAt === "object" && !Array.isArray(row.actualReceivedAt) ? row.actualReceivedAt : {};
  return { keyword: stringValue(row.keyword), actualReceivedAt: { from: range.from ?? "", to: range.to ?? "" }, supplierKeyword: stringValue(row.supplierKeyword) };
}
function toModelQuery(value: QueryPanelValue, missingDrugInspection: boolean, missingUpstreamDelivery: boolean) {
  const normalized = normalizePanelValue(value);
  const range = normalized.actualReceivedAt as QueryPanelRangeValue;
  const supplier = stringValue(normalized.supplierKeyword).trim();
  return { keyword: [stringValue(normalized.keyword), supplier].filter(Boolean).join(" "), receivedFrom: range.from ?? "", receivedTo: range.to ?? "", missingDrugInspection, missingUpstreamDelivery };
}
function textColumn(key: "receiptNo" | "purchaseOrderNo" | "supplierName", header: string, width: number, mono = false): DataGridColumn<InboundDocumentEntryRow> {
  return { key, header, width, sortable: true, sortValue: (row) => row[key], filterValue: (row) => row[key], filter: { type: "text" }, render: (row) => <span className={mono ? "font-mono" : undefined}>{row[key]}</span> };
}
function stringValue(value: QueryPanelValue[string]) { return typeof value === "string" ? value : ""; }
function formatDateTime(value: string | null) { return value ? new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short", timeZone: "Asia/Shanghai" }).format(new Date(value)) : "未实际收货"; }
function drugStatusLabel(status: InboundDocumentEntryRow["drugInspectionStatus"]) { return ({ pending_receipt: "待收货", pending_batch: "待批号", missing: "未录入", partial: "部分录入", complete: "已齐全" })[status]; }
function drugStatusVariant(status: InboundDocumentEntryRow["drugInspectionStatus"]): "pending" | "isolated" | "completed" { return status === "complete" ? "completed" : status === "partial" ? "isolated" : "pending"; }
function upstreamStatusLabel(status: InboundDocumentEntryRow["upstreamDeliveryStatus"]) { return status === "uploaded" ? "已上传" : "未上传"; }
function overallStatusLabel(row: InboundDocumentEntryRow) { return !row.actualReceivedAt ? "待收货" : row.drugInspectionStatus === "complete" && row.upstreamDeliveryStatus === "uploaded" ? "资料齐全" : "资料不齐"; }
function overallStatusVariant(row: InboundDocumentEntryRow): "pending" | "completed" | "isolated" { return !row.actualReceivedAt ? "pending" : overallStatusLabel(row) === "资料齐全" ? "completed" : "isolated"; }
function Field({ label, children }: { label: string; children: React.ReactNode }) { return <label className="grid gap-1 text-sm"><span>{label}</span>{children}</label>; }
function ImagePreview(props: { title: string; src: string; pending?: boolean; error?: boolean }) {
  return <figure className="grid min-h-48 content-start gap-2 rounded-md border bg-background p-2">
    <figcaption className="text-sm font-medium">{props.title}</figcaption>
    {props.src
      ? <img src={props.src} alt={props.title} className="max-h-72 w-full rounded object-contain" />
      : <div className="grid min-h-40 place-items-center text-sm text-muted-foreground">{props.error ? "预览生成失败" : props.pending ? "正在生成真实处理预览…" : "等待预览"}</div>}
  </figure>;
}
function VersionNote({ row, kind }: { row: InboundDocumentEntryRow; kind: EntryTab }) {
  const version = kind === "drug" ? row.drugInspectionVersion : row.upstreamVersion;
  return <div className="rounded-md border bg-muted/30 p-3 text-sm"><div className="font-medium">版本与修改记录</div><div className="mt-1 text-muted-foreground">{version ? `当前 v${version}；重新上传生成新版本，旧版本永久保留。` : "尚无版本记录。"}{row.lastModifiedReason ? ` 最近原因：${row.lastModifiedReason}` : ""}</div></div>;
}
