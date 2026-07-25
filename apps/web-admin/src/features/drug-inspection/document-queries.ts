import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api, putApiBinary } from "@/lib/api";

export type InboundDocumentEntry = components["schemas"]["InboundDocumentEntry"];
export type DrugInspectionReportVersion =
  components["schemas"]["DrugInspectionReportVersion"];
export type ReusableDrugInspectionReport =
  components["schemas"]["ReusableDrugInspectionReportResponse"];
export type UpstreamDeliveryDocumentVersion =
  components["schemas"]["UpstreamDeliveryDocumentVersion"];
export type DrugInspectionReviewQueueEntry =
  components["schemas"]["DrugInspectionReviewQueueEntry"];
export type DrugInspectionRequirementRule =
  components["schemas"]["DrugInspectionRequirementRule"];
export type DrugInspectionImagePreview =
  components["schemas"]["DrugInspectionImagePreviewResponse"];

export interface InboundDocumentFilters {
  receivedFrom: string;
  receivedTo: string;
  missingDrugInspection: boolean;
  missingUpstreamDelivery: boolean;
}

export interface SaveDrugInspectionInput {
  row: InboundDocumentEntry;
  batchNo: string;
  reportNo: string;
  file?: File;
  processingMode: "none" | "color_enhance" | "black_white_enhance";
  qualified: boolean;
  source: "upload" | "reuse";
  modificationReason?: string;
  attachmentId?: string;
}

export interface SaveUpstreamDeliveryInput {
  row: InboundDocumentEntry;
  asnIds: string[];
  files: File[];
  modificationReason?: string;
}

export const inboundDocumentsQueryKey = ["drug-inspection", "inbound-documents"] as const;
export const reviewQueueQueryKey = ["drug-inspection", "review-queue"] as const;
export const requirementRulesQueryKey =
  ["drug-inspection", "requirement-rules"] as const;

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto.randomUUID()}`;
}

async function listInboundDocuments(
  filters: InboundDocumentFilters,
): Promise<InboundDocumentEntry[]> {
  const result = await api.GET("/api/v1/drug-inspection/inbound-documents", {
    params: {
      query: {
        received_from: filters.receivedFrom || undefined,
        received_to: filters.receivedTo || undefined,
        missing_drug_inspection: filters.missingDrugInspection || undefined,
        missing_upstream_delivery: filters.missingUpstreamDelivery || undefined,
      },
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取入库资料失败", result.response.status);
  }
  return result.data.data;
}

async function findReusableReport(input: {
  productId: string;
  batchNo: string;
  asnId: string;
}): Promise<ReusableDrugInspectionReport | null> {
  const result = await api.GET("/api/v1/drug-inspection/reports/reusable", {
    params: {
      query: {
        product_id: input.productId,
        batch_no: input.batchNo,
        asn_id: input.asnId,
      },
    },
  });
  if (result.response.status === 404) return null;
  if (!result.data) {
    throw new ApiError(result.error, "查找可复用药检单失败", result.response.status);
  }
  return result.data;
}

async function findEditableVersion(input: {
  productId: string;
  batchNo: string;
  asnId: string;
}): Promise<DrugInspectionReportVersion | null> {
  const result = await api.GET("/api/v1/drug-inspection/report-versions/editable", {
    params: {
      query: {
        product_id: input.productId,
        batch_no: input.batchNo,
        asn_id: input.asnId,
      },
    },
  });
  if (result.response.status === 404) return null;
  if (!result.data) {
    throw new ApiError(result.error, "查找可编辑药检单草稿失败", result.response.status);
  }
  return result.data;
}

export async function uploadDrugInspectionAttachment(input: {
  file: File;
  entityId: string;
  entityType: string;
}): Promise<string> {
  const create = await api.POST("/api/v1/attachments/uploads", {
    params: {
      header: { "Idempotency-Key": idempotencyKey("web-h-file-create") },
    },
    body: {
      module: "M-DI",
      entity_type: input.entityType,
      entity_id: input.entityId,
      file_name: input.file.name,
      content_type: input.file.type,
      size_bytes: input.file.size,
    },
  });
  if (!create.data) {
    throw new ApiError(create.error, "创建附件上传会话失败", create.response.status);
  }
  const upload = await putApiBinary(create.data.upload_url, input.file);
  if (!upload.ok) {
    throw new ApiError(undefined, "上传附件正文失败", upload.status);
  }
  const confirm = await api.POST("/api/v1/attachments/confirm", {
    params: {
      header: { "Idempotency-Key": idempotencyKey("web-h-file-confirm") },
    },
    body: { upload_id: create.data.upload_id },
  });
  if (!confirm.data) {
    throw new ApiError(confirm.error, "确认附件失败", confirm.response.status);
  }
  return confirm.data.id;
}

export async function createDrugInspectionImagePreview(input: {
  attachmentId: string;
  processingMode: "none" | "color_enhance" | "black_white_enhance";
}): Promise<DrugInspectionImagePreview> {
  const result = await api.POST("/api/v1/drug-inspection/image-previews", {
    body: {
      attachment_id: input.attachmentId,
      processing_mode: input.processingMode,
    },
  });
  if (!result.data) {
    throw new ApiError(result.error, "生成药检图片预览失败", result.response.status);
  }
  return result.data;
}

async function saveDrugInspection(
  input: SaveDrugInspectionInput,
): Promise<DrugInspectionReportVersion | ReusableDrugInspectionReport> {
  const lookup = {
    productId: input.row.product_id,
    batchNo: input.batchNo,
    asnId: input.row.asn_id,
  };
  const [editable, reusable] = await Promise.all([
    input.source === "upload" ? findEditableVersion(lookup) : Promise.resolve(null),
    findReusableReport(lookup),
  ]);
  if (input.source === "reuse") {
    if (!reusable) {
      throw new ApiError(undefined, "当前商品批号没有可复用的已确认药检单", 404);
    }
    const reuse = await api.POST("/api/v1/drug-inspection/reports/{report_id}/reuse", {
      params: {
        path: { report_id: reusable.report_id },
        header: { "Idempotency-Key": idempotencyKey("web-di-reuse") },
      },
      body: { asn_id: input.row.asn_id, batch_no: input.batchNo },
    });
    if (!reuse.data) {
      throw new ApiError(reuse.error, "复用药检单失败", reuse.response.status);
    }
    return reusable;
  }
  if (!input.file) {
    throw new ApiError(undefined, "请选择药检单文件", 422);
  }
  if (!editable && reusable && !reusable.linked_to_asn) {
    throw new ApiError(
      undefined,
      "当前商品批号已有已确认药检单，请选择“复用已有药检单”",
      409,
    );
  }
  if (!editable && reusable?.linked_to_asn && !input.modificationReason?.trim()) {
    throw new ApiError(undefined, "重新上传当前批号的药检单必须填写修改原因", 422);
  }
  const attachmentId =
    input.attachmentId
    ?? await uploadDrugInspectionAttachment({
      file: input.file,
      entityId: input.row.asn_id,
      entityType: "drug_inspection_original",
    });
  const mutation = editable
    ? await api.PUT("/api/v1/drug-inspection/report-versions/{version_id}", {
      params: {
        path: { version_id: editable.id },
        header: { "Idempotency-Key": idempotencyKey("web-di-draft-update") },
      },
      body: {
        report_no: input.reportNo,
        original_file_id: attachmentId,
        processing_mode: input.processingMode,
        qualified: input.qualified,
      },
    })
    : reusable?.linked_to_asn
    ? await api.POST("/api/v1/drug-inspection/reports/{report_id}/corrections", {
      params: {
        path: { report_id: reusable.report_id },
        header: { "Idempotency-Key": idempotencyKey("web-di-correction") },
      },
      body: {
        report_no: input.reportNo,
        original_file_id: attachmentId,
        processing_mode: input.processingMode,
        qualified: input.qualified,
        modification_reason: input.modificationReason ?? "",
      },
    })
    : await api.POST("/api/v1/drug-inspection/report-versions", {
      params: {
        header: { "Idempotency-Key": idempotencyKey("web-di-create") },
      },
      body: {
        asn_id: input.row.asn_id,
        product_id: input.row.product_id,
        batch_no: input.batchNo,
        report_no: input.reportNo,
        original_file_id: attachmentId,
        source: "manual_upload",
        processing_mode: input.processingMode,
        qualified: input.qualified,
      },
    });
  if (!mutation.data) {
    throw new ApiError(mutation.error, "保存药检单版本失败", mutation.response.status);
  }
  const submit = await api.POST(
    "/api/v1/drug-inspection/report-versions/{version_id}/submit",
    {
      params: {
        path: { version_id: mutation.data.id },
        header: { "Idempotency-Key": idempotencyKey("web-di-submit") },
      },
    },
  );
  if (!submit.data) {
    throw new ApiError(submit.error, "提交药检单审核失败", submit.response.status);
  }
  return submit.data;
}

async function saveUpstreamDelivery(
  input: SaveUpstreamDeliveryInput,
): Promise<UpstreamDeliveryDocumentVersion> {
  const attachmentIds = await Promise.all(
    input.files.map((file) =>
      uploadDrugInspectionAttachment({
        file,
        entityId: input.row.asn_id,
        entityType: "upstream_delivery_document",
      }),
    ),
  );
  const result = await api.POST(
    "/api/v1/drug-inspection/upstream-delivery-document-versions",
    {
      params: {
        header: { "Idempotency-Key": idempotencyKey("web-di-upstream") },
      },
      body: {
        document_id: input.row.upstream_document_id ?? undefined,
        supplier_id: input.row.supplier_id,
        asn_ids: input.asnIds,
        attachment_ids: attachmentIds,
        modification_reason: input.modificationReason || undefined,
      },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "上传上游随货同行单失败", result.response.status);
  }
  return result.data;
}

async function listReviewQueue(): Promise<DrugInspectionReviewQueueEntry[]> {
  const result = await api.GET("/api/v1/drug-inspection/review-queue");
  if (!result.data) {
    throw new ApiError(result.error, "读取药检单审核队列失败", result.response.status);
  }
  return result.data;
}

async function listReportVersions(
  reportId: string,
): Promise<DrugInspectionReportVersion[]> {
  const result = await api.GET(
    "/api/v1/drug-inspection/reports/{report_id}/versions",
    { params: { path: { report_id: reportId } } },
  );
  if (!result.data) {
    throw new ApiError(result.error, "读取药检单版本记录失败", result.response.status);
  }
  return result.data;
}

async function reviewDrugInspection(input: {
  versionId: string;
  decision: "confirmed" | "rejected";
  comment?: string;
}): Promise<DrugInspectionReportVersion> {
  const result = await api.POST(
    "/api/v1/drug-inspection/report-versions/{version_id}/review",
    {
      params: {
        path: { version_id: input.versionId },
        header: { "Idempotency-Key": idempotencyKey("web-di-review") },
      },
      body: { decision: input.decision, comment: input.comment },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "审核药检单失败", result.response.status);
  }
  return result.data;
}

async function listRequirementRules(): Promise<DrugInspectionRequirementRule[]> {
  const result = await api.GET("/api/v1/drug-inspection/requirement-rules");
  if (!result.data) {
    throw new ApiError(result.error, "读取药检要求规则失败", result.response.status);
  }
  return result.data;
}

async function upsertRequirementRule(input: {
  specialDrugCategory: string;
  missingBehavior: "warning" | "block";
  enabled: boolean;
}): Promise<DrugInspectionRequirementRule> {
  const result = await api.PUT(
    "/api/v1/drug-inspection/requirement-rules/current",
    {
      params: {
        header: { "Idempotency-Key": idempotencyKey("web-di-requirement-rule") },
      },
      body: {
        special_drug_category: input.specialDrugCategory,
        missing_behavior: input.missingBehavior,
        enabled: input.enabled,
      },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "保存药检要求规则失败", result.response.status);
  }
  return result.data;
}

export async function getAttachmentDownloadUrl(attachmentId: string) {
  const result = await api.GET("/api/v1/attachments/{attachment_id}/url", {
    params: { path: { attachment_id: attachmentId } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "获取附件查看地址失败", result.response.status);
  }
  return result.data.url;
}

export function useInboundDocumentsQuery(filters: InboundDocumentFilters) {
  return useQuery<InboundDocumentEntry[], ApiError>({
    queryKey: [...inboundDocumentsQueryKey, filters],
    queryFn: () => listInboundDocuments(filters),
  });
}

export function useSaveDrugInspectionMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionReportVersion | ReusableDrugInspectionReport,
    ApiError,
    SaveDrugInspectionInput
  >({
    mutationFn: saveDrugInspection,
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: inboundDocumentsQueryKey }),
  });
}

export function useSaveUpstreamDeliveryMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    UpstreamDeliveryDocumentVersion,
    ApiError,
    SaveUpstreamDeliveryInput
  >({
    mutationFn: saveUpstreamDelivery,
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: inboundDocumentsQueryKey }),
  });
}

export function useDrugInspectionReviewQueueQuery() {
  return useQuery<DrugInspectionReviewQueueEntry[], ApiError>({
    queryKey: reviewQueueQueryKey,
    queryFn: listReviewQueue,
  });
}

export function useDrugInspectionVersionsQuery(reportId: string | null) {
  return useQuery<DrugInspectionReportVersion[], ApiError>({
    queryKey: ["drug-inspection", "reports", reportId, "versions"],
    queryFn: () => listReportVersions(reportId ?? ""),
    enabled: Boolean(reportId),
  });
}

export function useReviewDrugInspectionMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionReportVersion,
    ApiError,
    { versionId: string; decision: "confirmed" | "rejected"; comment?: string }
  >({
    mutationFn: reviewDrugInspection,
    onSuccess: (version) => {
      void queryClient.invalidateQueries({ queryKey: reviewQueueQueryKey });
      void queryClient.invalidateQueries({
        queryKey: ["drug-inspection", "reports", version.report_id, "versions"],
      });
      void queryClient.invalidateQueries({ queryKey: inboundDocumentsQueryKey });
    },
  });
}

export function useDrugInspectionRequirementRulesQuery() {
  return useQuery<DrugInspectionRequirementRule[], ApiError>({
    queryKey: requirementRulesQueryKey,
    queryFn: listRequirementRules,
  });
}

export function useUpsertDrugInspectionRequirementRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionRequirementRule,
    ApiError,
    {
      specialDrugCategory: string;
      missingBehavior: "warning" | "block";
      enabled: boolean;
    }
  >({
    mutationFn: upsertRequirementRule,
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: requirementRulesQueryKey }),
  });
}
