export type DrugInspectionDocumentStatus = "pending_receipt" | "pending_batch" | "missing" | "partial" | "complete";
export type UpstreamDeliveryDocumentStatus = "missing" | "uploaded";

export interface InboundDocumentEntryRow {
  id: string;
  receiptNo: string;
  purchaseOrderNo: string;
  ownerId: string;
  supplierId: string;
  supplierName: string;
  productId: string;
  productCode: string;
  productName: string;
  batchNos: string[];
  actualReceivedAt: string | null;
  drugInspectionStatus: DrugInspectionDocumentStatus;
  drugInspectionVersion: number;
  upstreamDeliveryStatus: UpstreamDeliveryDocumentStatus;
  upstreamVersion: number;
  upstreamDocumentId?: string;
  lastModifiedReason?: string;
  createdAt: string;
}

export interface InboundDocumentEntryQuery {
  keyword: string;
  receivedFrom: string;
  receivedTo: string;
  missingDrugInspection: boolean;
  missingUpstreamDelivery: boolean;
}

interface ReceivingOrderSource {
  id: string;
  owner_id: string;
  receipt_no: string;
  document_type: string;
  external_ref?: string | null;
  supplier_id?: string | null;
  status: string;
  updated_at: string;
  created_at: string;
  lines: Array<{ product_code: string; batch_no?: string | null }>;
}

interface UploadFileLike {
  name: string;
  size: number;
  type: string;
}

const WMS_TIME_ZONE = "Asia/Shanghai";

export function toWmsBusinessDate(value: Date | string) {
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: WMS_TIME_ZONE,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(date);
  const valueOf = (type: Intl.DateTimeFormatPartTypes) => parts.find((part) => part.type === type)?.value ?? "";
  return `${valueOf("year")}-${valueOf("month")}-${valueOf("day")}`;
}

export function defaultInboundDocumentQuery(today = toWmsBusinessDate(new Date())): InboundDocumentEntryQuery {
  const from = new Date(`${today}T00:00:00.000Z`);
  from.setUTCDate(from.getUTCDate() - 89);
  return {
    keyword: "",
    receivedFrom: from.toISOString().slice(0, 10),
    receivedTo: today,
    missingDrugInspection: false,
    missingUpstreamDelivery: false,
  };
}

export function buildConfirmationDocumentRows(orders: ReceivingOrderSource[]): InboundDocumentEntryRow[] {
  return orders
    .filter((order) => order.document_type === "purchase_inbound")
    .map((order, index) => {
      const received = !["created", "released"].includes(order.status);
      const batchNos = order.lines.flatMap((line) => line.batch_no ? [line.batch_no] : []);
      const confirmationBatchNos = received && batchNos.length === 0
        ? [`BATCH-${order.receipt_no.replace(/\W+/g, "-")}`]
        : batchNos;
      const drugStatuses: DrugInspectionDocumentStatus[] = ["missing", "partial", "complete"];
      const drugInspectionStatus = received
        ? drugStatuses[index % drugStatuses.length] ?? "missing"
        : "pending_receipt";
      const upstreamDeliveryStatus: UpstreamDeliveryDocumentStatus = received && index % 2 === 1
        ? "uploaded"
        : "missing";
      return {
        id: order.id,
        receiptNo: order.receipt_no,
        purchaseOrderNo: order.external_ref || `PO-${order.receipt_no}`,
        ownerId: order.owner_id,
        supplierId: order.supplier_id || "supplier-unassigned",
        supplierName: order.supplier_id ? `供应商 ${order.supplier_id.slice(-4)}` : "供应商待维护",
        productId: order.lines[0]?.product_code ?? "-",
        productCode: order.lines[0]?.product_code ?? "-",
        productName: order.lines[0]?.product_code ?? "-",
        batchNos: confirmationBatchNos,
        actualReceivedAt: received ? order.updated_at : null,
        drugInspectionStatus,
        drugInspectionVersion: ["partial", "complete"].includes(drugInspectionStatus) ? 1 : 0,
        upstreamDeliveryStatus,
        upstreamVersion: upstreamDeliveryStatus === "uploaded" ? 1 : 0,
        createdAt: order.created_at,
      };
    });
}

export function filterInboundDocumentRows(
  rows: InboundDocumentEntryRow[],
  query: InboundDocumentEntryQuery,
) {
  const keyword = query.keyword.trim().toLocaleLowerCase();
  return rows.filter((row) => {
    const receivedDate = row.actualReceivedAt ? toWmsBusinessDate(row.actualReceivedAt) : "";
    const inDateRange = (!query.receivedFrom || receivedDate >= query.receivedFrom)
      && (!query.receivedTo || receivedDate <= query.receivedTo);
    const matchesKeyword = !keyword || [
      row.receiptNo,
      row.purchaseOrderNo,
      row.supplierId,
      row.supplierName,
      row.productCode,
      ...row.batchNos,
    ].join(" ").toLocaleLowerCase().includes(keyword);
    const noQuickFilter = !query.missingDrugInspection && !query.missingUpstreamDelivery;
    const matchesMissing = noQuickFilter
      || (query.missingDrugInspection && ["missing", "partial"].includes(row.drugInspectionStatus))
      || (query.missingUpstreamDelivery && row.upstreamDeliveryStatus === "missing");
    return inDateRange && matchesKeyword && matchesMissing;
  });
}

export function validateUpstreamDeliveryFiles(files: UploadFileLike[]) {
  if (files.length === 0) return "请选择一个 PDF 或多张 JPG";
  if (files.some((file) => file.size > 5 * 1024 * 1024)) return "每个文件必须小于或等于 5MB";
  const isPdf = (file: UploadFileLike) => file.type === "application/pdf" || /\.pdf$/i.test(file.name);
  const isJpg = (file: UploadFileLike) => file.type === "image/jpeg" || /\.jpe?g$/i.test(file.name);
  if (files.length === 1 && isPdf(files[0])) return null;
  if (files.every(isJpg)) return null;
  return "只能上传一个 PDF，或上传一张及以上 JPG，不能混合上传";
}

export function validateDrugInspectionFile(file: UploadFileLike | null) {
  if (!file) return "请选择药检单文件";
  const isPdf = file.type === "application/pdf" || /\.pdf$/i.test(file.name);
  const isImage = ["image/jpeg", "image/png"].includes(file.type) || /\.(jpe?g|png)$/i.test(file.name);
  if (!isPdf && !isImage) return "药检单只支持 PDF、JPG 或 PNG";
  const limit = isPdf ? 50 : 5;
  return file.size <= limit * 1024 * 1024 ? null : `${isPdf ? "PDF" : "JPG/PNG"} 不能超过 ${limit}MB`;
}
