import type { IncomingMessage, ServerResponse } from "node:http";

import {
  allDevOrders,
  asString,
  readJsonBody,
  sendError,
  sendJson,
} from "./web-admin-dev-mock-core-common";
import {
  devOwnerId,
  devSeedProducts,
  devSupplier,
  devUserId,
} from "./web-admin-dev-mock-model";

interface DevUpload {
  entityId: string;
  entityType: string;
  fileName: string;
  contentType: string;
  sizeBytes: number;
}

const uploads = new Map<string, DevUpload>();
const upstreamAsnIds = new Set<string>();

export async function handleDrugInspectionDocumentDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (req.method === "GET" && pathname === "/api/v1/drug-inspection/inbound-documents") {
    const data = inboundDocumentRows();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/attachments/uploads") {
    const body = await readJsonBody(req);
    const uploadId = crypto.randomUUID();
    uploads.set(uploadId, {
      entityId: asString(body.entity_id, crypto.randomUUID()),
      entityType: asString(body.entity_type, "drug_inspection"),
      fileName: asString(body.file_name, "attachment.bin"),
      contentType: asString(body.content_type, "application/octet-stream"),
      sizeBytes: typeof body.size_bytes === "number" ? body.size_bytes : 0,
    });
    sendJson(res, 200, {
      upload_id: uploadId,
      upload_url: `/api/v1/attachments/uploads/${uploadId}/content?token=dev`,
      expires_at: new Date(Date.now() + 5 * 60_000).toISOString(),
    });
    return true;
  }

  const uploadContent = pathname.match(/^\/api\/v1\/attachments\/uploads\/([^/]+)\/content$/);
  if (req.method === "PUT" && uploadContent) {
    if (!uploads.has(uploadContent[1])) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Upload session not found");
      return true;
    }
    for await (const _chunk of req) {
      // Drain the mock upload body before acknowledging it.
    }
    res.statusCode = 204;
    res.end();
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/attachments/confirm") {
    const body = await readJsonBody(req);
    const uploadId = asString(body.upload_id, "");
    const upload = uploads.get(uploadId);
    if (!upload) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Upload session not found");
      return true;
    }
    uploads.delete(uploadId);
    sendJson(res, 200, {
      id: crypto.randomUUID(),
      owner_id: devOwnerId,
      module: "M-DI",
      entity_type: upload.entityType,
      entity_id: upload.entityId,
      file_name: upload.fileName,
      content_type: upload.contentType,
      size_bytes: upload.sizeBytes,
      sha256: "dev-mock-sha256",
      uploaded_by: devUserId,
      created_at: new Date().toISOString(),
    });
    return true;
  }

  if (
    req.method === "POST"
    && pathname === "/api/v1/drug-inspection/upstream-delivery-document-versions"
  ) {
    const body = await readJsonBody(req);
    const asnIds = stringArray(body.asn_ids);
    const attachmentIds = stringArray(body.attachment_ids);
    for (const asnId of asnIds) upstreamAsnIds.add(asnId);
    const now = new Date().toISOString();
    sendJson(res, 200, {
      id: crypto.randomUUID(),
      document_id: asString(body.document_id, crypto.randomUUID()),
      owner_id: devOwnerId,
      version_number: 1,
      modification_reason: asString(body.modification_reason, "") || null,
      attachment_ids: attachmentIds,
      asn_ids: asnIds,
      uploaded_by: devUserId,
      created_at: now,
    });
    return true;
  }

  return false;
}

function inboundDocumentRows() {
  const statuses = ["missing", "partial", "complete"];
  return allDevOrders()
    .filter((order) => order.document_type === "purchase_inbound" && order.status !== "released")
    .slice(0, 6)
    .map((order, index) => {
      const product = devSeedProducts[index % devSeedProducts.length];
      const drugInspectionStatus = statuses[index % statuses.length] ?? "missing";
      const upstreamUploaded = index % 2 === 1 || upstreamAsnIds.has(order.id);
      return {
        asn_id: order.id,
        receipt_no: order.receipt_no,
        purchase_order_no: order.external_ref ?? `PO-${order.receipt_no}`,
        owner_id: devOwnerId,
        supplier_id: devSupplier.id,
        supplier_name: devSupplier.supplier_name,
        product_id: product?.id ?? "00000000-0000-0000-0000-000000001001",
        product_code: product?.product_code ?? "-",
        product_name: product?.product_name ?? "-",
        batch_nos: [
          `BATCH-${order.receipt_no.slice(-4)}-A`,
          ...(index === 1 ? [`BATCH-${order.receipt_no.slice(-4)}-B`] : []),
        ],
        actual_received_at: order.updated_at,
        drug_inspection_status: drugInspectionStatus,
        drug_inspection_version: drugInspectionStatus === "missing" ? 0 : 1,
        upstream_delivery_status: upstreamUploaded ? "uploaded" : "missing",
        upstream_version: upstreamUploaded ? 1 : 0,
        upstream_document_id: upstreamUploaded ? order.id : null,
        created_at: order.created_at,
      };
    });
}

function stringArray(value: unknown) {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}
