import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

type PrintFieldLibrarySummary = components["schemas"]["PrintFieldLibrarySummary"];
type PrintFieldDefinition = components["schemas"]["PrintFieldDefinition"];
type PrintTemplateSummary = components["schemas"]["PrintTemplateSummary"];
type SystemDictionaryItem = components["schemas"]["SystemDictionaryItem"];

export type GeneratePrintFieldLibraryDraftRequest = components["schemas"]["GeneratePrintFieldLibraryDraftRequest"];
export type UpdatePrintFieldDefinitionRequest = components["schemas"]["UpdatePrintFieldDefinitionRequest"];
export type PrintTemplateBinding = components["schemas"]["PrintTemplateBinding"];
export type PrintTemplateVersion = components["schemas"]["PrintTemplateVersion"];
export type ResolvePrintTemplateRequest = components["schemas"]["ResolvePrintTemplateRequest"];
export type ResolvePrintTemplateResponse = components["schemas"]["ResolvePrintTemplateResponse"];
export type PrintTemplatePreviewRequest = components["schemas"]["PrintTemplatePreviewRequest"];
export type PrintTemplatePreviewResponse = components["schemas"]["PrintTemplatePreviewResponse"];
export type PrintTemplatePrintRequest = components["schemas"]["PrintTemplatePrintRequest"];
export type SavePrintTemplateRequest = components["schemas"]["SavePrintTemplateRequest"];

export interface PrintFieldLibraryRow {
  id: string;
  libraryCode: string;
  libraryName: string;
  businessModule: string;
  sourceSchema: string;
  latestVersionId: string;
  versionNo: number;
  publishedVersionId: string | null;
  publishedVersionNo: number | null;
  fieldCount: number;
  createdAt: string;
  createdBy: string;
  publishedAt: string | null;
  publishedBy: string | null;
  status: "draft" | "published";
  statusLabel: string;
  searchText: string;
}

export interface PrintFieldDefinitionRow {
  id: string;
  libraryVersionId: string;
  fieldPath: string;
  fieldType: string;
  sourceSchema: string;
  displayName: string;
  groupCode: string;
  groupName: string;
  description: string;
  exampleValue: unknown;
  printable: boolean;
  sensitive: boolean;
  maskingRule: string | null;
  formattingRule: string | null;
  supportsBarcode: boolean;
  supportsQrcode: boolean;
  isTableDetail: boolean;
  sortOrder: number;
}

export interface PrintTemplateTypeRow {
  id: string;
  code: string;
  name: string;
  enabled: boolean;
  fieldLibraryCode: string;
  businessModule: string;
  businessDirection: string;
  paperType: string;
  defaultScope: string;
  searchText: string;
}

export interface PrintTemplateRow {
  id: string;
  templateCode: string;
  templateName: string;
  templateTypeCode: string;
  scope: "global" | "owner";
  scopeLabel: string;
  enabled: boolean;
  isDefault: boolean;
  latestVersionId: string;
  latestVersionNo: number;
  latestVersionStatus: string;
  fieldLibraryVersionId: string;
  designerVersion: string;
  createdAt: string;
  publishedAt: string | null;
  updatedAt: string;
  statusLabel: string;
  searchText: string;
}

export const printTemplateQueryKey = ["print-template"] as const;

export function usePrintFieldLibrariesQuery() {
  return useQuery<PrintFieldLibraryRow[], ApiError>({
    queryKey: [...printTemplateQueryKey, "field-libraries"],
    queryFn: listPrintFieldLibraries,
  });
}

export function usePrintTemplateTypesQuery() {
  return useQuery<PrintTemplateTypeRow[], ApiError>({
    queryKey: [...printTemplateQueryKey, "template-types"],
    queryFn: listPrintTemplateTypes,
  });
}

export function usePrintTemplatesQuery() {
  return useQuery<PrintTemplateRow[], ApiError>({
    queryKey: [...printTemplateQueryKey, "templates"],
    queryFn: listPrintTemplates,
  });
}

export function usePrintFieldDefinitionsQuery(libraryVersionId: string | null) {
  return useQuery<PrintFieldDefinitionRow[], ApiError>({
    queryKey: [...printTemplateQueryKey, "field-definitions", libraryVersionId],
    queryFn: () => listPrintFieldDefinitions(libraryVersionId ?? ""),
    enabled: Boolean(libraryVersionId),
  });
}

export function useSavePrintTemplateMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: savePrintTemplate,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: printTemplateQueryKey });
    },
  });
}

export function useGeneratePrintFieldLibraryDraftMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: generatePrintFieldLibraryDraft,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: printTemplateQueryKey });
    },
  });
}

export function useUpdatePrintFieldDefinitionMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: updatePrintFieldDefinition,
    onSuccess: (_, variables) => {
      void queryClient.invalidateQueries({
        queryKey: [...printTemplateQueryKey, "field-definitions", variables.libraryVersionId],
      });
      void queryClient.invalidateQueries({ queryKey: [...printTemplateQueryKey, "field-libraries"] });
    },
  });
}

export function usePublishPrintFieldLibraryMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: publishPrintFieldLibrary,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: printTemplateQueryKey });
    },
  });
}

export function useResolvePrintTemplateMutation() {
  return useMutation<ResolvePrintTemplateResponse, ApiError, ResolvePrintTemplateRequest>({
    mutationFn: resolvePrintTemplate,
  });
}

export function usePrintTemplateVersionsMutation() {
  return useMutation<PrintTemplateVersion[], ApiError, string>({
    mutationFn: listPrintTemplateVersions,
  });
}

export function usePreviewPrintTemplateMutation() {
  return useMutation<PrintTemplatePreviewResponse, ApiError, PrintTemplatePreviewRequest>({
    mutationFn: previewPrintTemplate,
  });
}

export function useRecordPrintTemplateMutation() {
  return useMutation({
    mutationFn: recordPrintTemplate,
  });
}

async function listPrintFieldLibraries(): Promise<PrintFieldLibraryRow[]> {
  const result = await api.GET("/api/v1/print-templates/field-libraries");
  if (!result.data) {
    throw new ApiError(result.error, "读取打印字段库失败", result.response.status);
  }
  return result.data.data.map(printFieldLibraryRow);
}

async function listPrintTemplateTypes(): Promise<PrintTemplateTypeRow[]> {
  const result = await api.GET("/api/v1/system-dictionaries/{dict_code}/items", {
    params: { path: { dict_code: "print_template_type" } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取打印模板类型失败", result.response.status);
  }
  return result.data.data.map(printTemplateTypeRow);
}

async function listPrintTemplates(): Promise<PrintTemplateRow[]> {
  const result = await api.GET("/api/v1/print-templates/templates");
  if (!result.data) {
    throw new ApiError(result.error, "读取打印模板失败", result.response.status);
  }
  return result.data.data.map(printTemplateRow);
}

async function listPrintFieldDefinitions(libraryVersionId: string): Promise<PrintFieldDefinitionRow[]> {
  const result = await api.GET("/api/v1/print-templates/field-libraries/{version_id}/fields", {
    params: { path: { version_id: libraryVersionId } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取打印字段定义失败", result.response.status);
  }
  return result.data.data.map(printFieldDefinitionRow);
}

async function savePrintTemplate(request: SavePrintTemplateRequest) {
  const result = await api.POST("/api/v1/print-templates/templates", {
    body: request,
    params: { header: { "Idempotency-Key": idempotencyKey("web-h9-template-save") } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "保存打印模板失败", result.response.status);
  }
  return result.data;
}

async function generatePrintFieldLibraryDraft(request: GeneratePrintFieldLibraryDraftRequest) {
  const result = await api.POST("/api/v1/print-templates/field-libraries/drafts", {
    body: request,
    params: { header: { "Idempotency-Key": idempotencyKey("web-h9-field-library-draft") } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "生成字段库草稿失败", result.response.status);
  }
  return result.data;
}

async function updatePrintFieldDefinition({
  libraryVersionId,
  fieldId,
  body,
}: {
  libraryVersionId: string;
  fieldId: string;
  body: UpdatePrintFieldDefinitionRequest;
}) {
  const result = await api.PATCH(
    "/api/v1/print-templates/field-libraries/{version_id}/fields/{field_id}",
    {
      body,
      params: {
        path: { version_id: libraryVersionId, field_id: fieldId },
        header: { "Idempotency-Key": idempotencyKey("web-h9-field-definition") },
      },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "保存字段元数据失败", result.response.status);
  }
  return result.data;
}

async function publishPrintFieldLibrary(libraryVersionId: string) {
  const result = await api.POST(
    "/api/v1/print-templates/field-libraries/{version_id}/publish",
    {
      params: {
        path: { version_id: libraryVersionId },
        header: { "Idempotency-Key": idempotencyKey("web-h9-field-library-publish") },
      },
    },
  );
  if (!result.data) {
    throw new ApiError(result.error, "发布字段库失败", result.response.status);
  }
  return result.data;
}

async function resolvePrintTemplate(request: ResolvePrintTemplateRequest) {
  const result = await api.POST("/api/v1/print-templates/resolve", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "解析打印模板失败", result.response.status);
  }
  return result.data;
}

async function listPrintTemplateVersions(templateId: string) {
  const result = await api.GET("/api/v1/print-templates/templates/{template_id}/versions", {
    params: { path: { template_id: templateId } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "读取打印模板版本历史失败", result.response.status);
  }
  return result.data.data;
}

async function previewPrintTemplate(request: PrintTemplatePreviewRequest) {
  const result = await api.POST("/api/v1/print-templates/preview", { body: request });
  if (!result.data) {
    throw new ApiError(result.error, "预览打印模板失败", result.response.status);
  }
  return result.data;
}

async function recordPrintTemplate(request: PrintTemplatePrintRequest) {
  const result = await api.POST("/api/v1/print-templates/print", {
    body: request,
    params: { header: { "Idempotency-Key": idempotencyKey("web-h9-print") } },
  });
  if (!result.data) {
    throw new ApiError(result.error, "记录打印结果失败", result.response.status);
  }
  return result.data;
}

function printFieldLibraryRow(row: PrintFieldLibrarySummary): PrintFieldLibraryRow {
  return {
    id: row.id,
    libraryCode: row.library_code,
    libraryName: row.library_name,
    businessModule: row.business_module,
    sourceSchema: row.source_schema,
    latestVersionId: row.latest_version_id,
    versionNo: row.version_no,
    publishedVersionId: row.latest_published_version_id ?? null,
    publishedVersionNo: row.latest_published_version_no ?? null,
    fieldCount: row.field_count,
    createdAt: row.created_at,
    createdBy: row.created_by,
    publishedAt: row.published_at ?? null,
    publishedBy: row.published_by ?? null,
    status: row.latest_version_status === "published" ? "published" : "draft",
    statusLabel: row.latest_version_status === "published" ? "已发布" : "草稿",
    searchText: [
      row.library_code,
      row.library_name,
      row.source_schema,
      row.latest_version_id,
      row.business_module,
      row.latest_version_status,
      row.published_by ?? "",
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function printFieldDefinitionRow(row: PrintFieldDefinition): PrintFieldDefinitionRow {
  return {
    id: row.id,
    libraryVersionId: row.library_version_id,
    fieldPath: row.field_path,
    fieldType: row.field_type,
    sourceSchema: row.source_schema,
    displayName: row.display_name,
    groupCode: row.group_code,
    groupName: row.group_name,
    description: row.description,
    exampleValue: row.example_value,
    printable: row.printable,
    sensitive: row.sensitive,
    maskingRule: row.masking_rule ?? null,
    formattingRule: row.formatting_rule ?? null,
    supportsBarcode: row.supports_barcode,
    supportsQrcode: row.supports_qrcode,
    isTableDetail: row.is_table_detail,
    sortOrder: row.sort_order,
  };
}

function printTemplateTypeRow(row: SystemDictionaryItem): PrintTemplateTypeRow {
  const fieldLibraryCode = paramText(row.params, "field_library_code");
  const businessModule = paramText(row.params, "business_module");
  const businessDirection = paramText(row.params, "business_direction");
  const paperType = paramText(row.params, "paper_type");
  const defaultScope = paramText(row.params, "default_scope");
  return {
    id: row.id,
    code: row.item_code,
    name: row.item_name,
    enabled: row.enabled,
    fieldLibraryCode,
    businessModule,
    businessDirection,
    paperType,
    defaultScope,
    searchText: [
      row.item_code,
      row.item_name,
      fieldLibraryCode,
      businessModule,
      businessDirection,
      paperType,
      defaultScope,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function printTemplateRow(row: PrintTemplateSummary): PrintTemplateRow {
  const statusLabel = row.enabled ? "启用" : "停用";
  return {
    id: row.id,
    templateCode: row.template_code,
    templateName: row.template_name,
    templateTypeCode: row.template_type_code,
    scope: row.scope,
    scopeLabel: row.scope === "owner" ? "货主" : "全局",
    enabled: row.enabled,
    isDefault: row.is_default,
    latestVersionId: row.latest_version_id,
    latestVersionNo: row.latest_version_no,
    latestVersionStatus: row.latest_version_status,
    fieldLibraryVersionId: row.field_library_version_id,
    designerVersion: row.designer_version,
    createdAt: row.created_at,
    publishedAt: row.published_at ?? null,
    updatedAt: row.updated_at,
    statusLabel,
    searchText: [
      row.template_code,
      row.template_name,
      row.template_type_code,
      row.scope,
      statusLabel,
      row.latest_version_no,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function paramText(params: Record<string, unknown>, key: string) {
  const value = params[key];
  return typeof value === "string" ? value : "";
}

function idempotencyKey(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}
