import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";
import { uploadDrugInspectionAttachment } from "./document-queries";

export type DrugInspectionStampVersion =
  components["schemas"]["DrugInspectionStampVersion"];
export type DrugInspectionCustomerCopyJob =
  components["schemas"]["DrugInspectionCustomerCopyJob"];
export type DrugInspectionProcessingRuleVersion =
  components["schemas"]["DrugInspectionProcessingRuleVersion"];

export interface CreateStampInput {
  file: File;
  relativeX: number;
  relativeY: number;
  relativeWidth: number;
}

const stampQueryKey = ["drug-inspection", "stamp-versions"] as const;
const copyJobQueryKey = ["drug-inspection", "customer-copy-jobs"] as const;
const processingRuleQueryKey =
  ["drug-inspection", "processing-rule-versions"] as const;

function idempotencyKey(prefix: string) {
  return `${prefix}-${globalThis.crypto.randomUUID()}`;
}

async function listStampVersions(): Promise<DrugInspectionStampVersion[]> {
  const result = await api.GET("/api/v1/drug-inspection/stamp-versions");
  if (!result.data) {
    throw new ApiError(result.error, "读取药检图章版本失败", result.response.status);
  }
  return result.data;
}

async function createStampVersion(
  input: CreateStampInput,
): Promise<DrugInspectionStampVersion> {
  const attachmentId = await uploadDrugInspectionAttachment({
    file: input.file,
    entityId: globalThis.crypto.randomUUID(),
    entityType: "drug_inspection_stamp",
  });
  const create = await api.POST("/api/v1/drug-inspection/stamp-versions", {
    params: {
      header: { "Idempotency-Key": idempotencyKey("web-di-stamp-create") },
    },
    body: {
      png_attachment_id: attachmentId,
      relative_x: input.relativeX,
      relative_y: input.relativeY,
      relative_width: input.relativeWidth,
    },
  });
  if (!create.data) {
    throw new ApiError(create.error, "保存图章版本失败", create.response.status);
  }
  const submit = await api.POST(
    "/api/v1/drug-inspection/stamp-versions/{version_id}/submit",
    {
      params: {
        path: { version_id: create.data.id },
        header: { "Idempotency-Key": idempotencyKey("web-di-stamp-submit") },
      },
    },
  );
  if (!submit.data) {
    throw new ApiError(submit.error, "提交图章审核失败", submit.response.status);
  }
  return submit.data;
}

async function reviewStamp(input: {
  versionId: string;
  decision: "published" | "rejected";
  comment?: string;
}): Promise<DrugInspectionStampVersion> {
  const result = await api.POST(
    "/api/v1/drug-inspection/stamp-versions/{version_id}/review",
    {
      params: {
        path: { version_id: input.versionId },
        header: { "Idempotency-Key": idempotencyKey("web-di-stamp-review") },
      },
      body: { decision: input.decision, comment: input.comment },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "审核图章版本失败", result.response.status);
  }
  return result.data;
}

async function listCopyJobs(): Promise<DrugInspectionCustomerCopyJob[]> {
  const result = await api.GET("/api/v1/drug-inspection/customer-copy-jobs");
  if (!result.data) {
    throw new ApiError(result.error, "读取客户副本任务失败", result.response.status);
  }
  return result.data;
}

async function listProcessingRules(): Promise<DrugInspectionProcessingRuleVersion[]> {
  const result = await api.GET(
    "/api/v1/drug-inspection/processing-rule-versions",
  );
  if (!result.data) {
    throw new ApiError(result.error, "读取图像处理规则版本失败", result.response.status);
  }
  return result.data;
}

async function publishProcessingRule(
  applyScope: "future_only" | "reprocess_current",
): Promise<DrugInspectionProcessingRuleVersion> {
  const result = await api.POST(
    "/api/v1/drug-inspection/processing-rule-versions",
    {
      params: {
        header: { "Idempotency-Key": idempotencyKey("web-di-processing-rule") },
      },
      body: { apply_scope: applyScope },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "发布图像处理规则失败", result.response.status);
  }
  return result.data;
}

async function approveOversize(input: {
  jobId: string;
  reason: string;
}): Promise<DrugInspectionCustomerCopyJob> {
  const result = await api.POST(
    "/api/v1/drug-inspection/customer-copy-jobs/{job_id}/oversize-approval",
    {
      params: {
        path: { job_id: input.jobId },
        header: { "Idempotency-Key": idempotencyKey("web-di-copy-oversize") },
      },
      body: { reason: input.reason },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "批准超限副本失败", result.response.status);
  }
  return result.data;
}

export function useDrugInspectionStampVersionsQuery() {
  return useQuery<DrugInspectionStampVersion[], ApiError>({
    queryKey: stampQueryKey,
    queryFn: listStampVersions,
  });
}

export function useCreateDrugInspectionStampMutation() {
  const queryClient = useQueryClient();
  return useMutation<DrugInspectionStampVersion, ApiError, CreateStampInput>({
    mutationFn: createStampVersion,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: stampQueryKey }),
  });
}

export function useReviewDrugInspectionStampMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionStampVersion,
    ApiError,
    { versionId: string; decision: "published" | "rejected"; comment?: string }
  >({
    mutationFn: reviewStamp,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: stampQueryKey }),
  });
}

export function useDrugInspectionCopyJobsQuery() {
  return useQuery<DrugInspectionCustomerCopyJob[], ApiError>({
    queryKey: copyJobQueryKey,
    queryFn: listCopyJobs,
    refetchInterval: 3_000,
  });
}

export function useDrugInspectionProcessingRulesQuery() {
  return useQuery<DrugInspectionProcessingRuleVersion[], ApiError>({
    queryKey: processingRuleQueryKey,
    queryFn: listProcessingRules,
  });
}

export function usePublishDrugInspectionProcessingRuleMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionProcessingRuleVersion,
    ApiError,
    "future_only" | "reprocess_current"
  >({
    mutationFn: publishProcessingRule,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: processingRuleQueryKey });
      void queryClient.invalidateQueries({ queryKey: copyJobQueryKey });
    },
  });
}

export function useApproveDrugInspectionCopyOversizeMutation() {
  const queryClient = useQueryClient();
  return useMutation<
    DrugInspectionCustomerCopyJob,
    ApiError,
    { jobId: string; reason: string }
  >({
    mutationFn: approveOversize,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: copyJobQueryKey }),
  });
}
