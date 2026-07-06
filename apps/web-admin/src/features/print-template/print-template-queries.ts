import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

type PrintFieldLibrarySummary = components["schemas"]["PrintFieldLibrarySummary"];
type SystemDictionaryItem = components["schemas"]["SystemDictionaryItem"];

export interface PrintFieldLibraryRow {
  id: string;
  libraryCode: string;
  libraryName: string;
  sourceSchema: string;
  latestVersionId: string;
  versionNo: number;
  fieldCount: number;
  createdAt: string;
  publishedAt: string;
  publishedBy: string;
  status: "published";
  statusLabel: string;
  searchText: string;
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

function printFieldLibraryRow(row: PrintFieldLibrarySummary): PrintFieldLibraryRow {
  return {
    id: row.id,
    libraryCode: row.library_code,
    libraryName: row.library_name,
    sourceSchema: row.source_schema,
    latestVersionId: row.latest_version_id,
    versionNo: row.version_no,
    fieldCount: row.field_count,
    createdAt: row.created_at,
    publishedAt: row.published_at,
    publishedBy: row.published_by,
    status: "published",
    statusLabel: "已发布",
    searchText: [
      row.library_code,
      row.library_name,
      row.source_schema,
      row.latest_version_id,
      row.published_by,
    ]
      .join(" ")
      .toLowerCase(),
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

function paramText(params: Record<string, unknown>, key: string) {
  const value = params[key];
  return typeof value === "string" ? value : "";
}
