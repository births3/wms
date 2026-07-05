import { useQuery } from "@tanstack/react-query";
import type { components } from "@wms/api-client";

import { ApiError } from "@/features/auth/auth-queries";
import { api } from "@/lib/api";

type PrintFieldLibrarySummary = components["schemas"]["PrintFieldLibrarySummary"];

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

export const printTemplateQueryKey = ["print-template"] as const;

export function usePrintFieldLibrariesQuery() {
  return useQuery<PrintFieldLibraryRow[], ApiError>({
    queryKey: [...printTemplateQueryKey, "field-libraries"],
    queryFn: listPrintFieldLibraries,
  });
}

async function listPrintFieldLibraries(): Promise<PrintFieldLibraryRow[]> {
  const result = await api.GET("/api/v1/print-templates/field-libraries");
  if (!result.data) {
    throw new ApiError(result.error, "读取打印字段库失败", result.response.status);
  }
  return result.data.data.map(printFieldLibraryRow);
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
