import * as React from "react";
import {
  DataGrid,
  PageHeader,
  QueryPanel,
  StatusBadge,
  TreeCatalog,
  buildQueryPanelSummaryItems,
  cn,
  type DataGridColumn,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
  type TreeCatalogNode,
} from "@wms/ui";

import {
  usePrintFieldLibrariesQuery,
  usePrintTemplateTypesQuery,
  type PrintFieldLibraryRow,
  type PrintTemplateTypeRow,
} from "@/features/print-template/print-template-queries";

const h9PrintTemplateQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "搜索字段库编码、名称或发布人",
    ariaLabel: "搜索打印字段库",
  },
  {
    key: "sourceSchema",
    label: "来源模型",
    type: "text",
    placeholder: "例如 ReceivingOrder",
    ariaLabel: "搜索来源模型",
  },
];
const h9PrintTemplateCoreQueryFieldKeys = ["keyword", "sourceSchema"];

const columns: DataGridColumn<PrintFieldLibraryRow>[] = [
  {
    key: "libraryCode",
    header: "字段库编码",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.libraryCode,
    filterValue: (row) => row.libraryCode,
    copyValue: (row) => row.libraryCode,
    filter: { type: "text" },
  },
  {
    key: "libraryName",
    header: "字段库名称",
    width: 220,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.libraryName,
    filterValue: (row) => row.libraryName,
    copyValue: (row) => row.libraryName,
    filter: { type: "text" },
  },
  {
    key: "sourceSchema",
    header: "来源模型",
    width: 180,
    minWidth: 140,
    mono: true,
    sortable: true,
    sortValue: (row) => row.sourceSchema,
    filterValue: (row) => row.sourceSchema,
    copyValue: (row) => row.sourceSchema,
    filter: { type: "text" },
  },
  {
    key: "versionNo",
    header: "最新版本",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.versionNo,
    filterValue: (row) => row.versionNo,
    copyValue: (row) => `v${row.versionNo}`,
    filter: { type: "numberRange" },
    render: (row) => `v${row.versionNo}`,
  },
  {
    key: "fieldCount",
    header: "字段数",
    width: 110,
    minWidth: 90,
    sortable: true,
    sortValue: (row) => row.fieldCount,
    filterValue: (row) => row.fieldCount,
    copyValue: (row) => String(row.fieldCount),
    filter: { type: "numberRange" },
  },
  {
    key: "status",
    header: "状态",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.statusLabel,
    filterValue: (row) => row.status,
    copyValue: (row) => row.statusLabel,
    filter: { type: "multiSelect", options: [{ label: "已发布", value: "published" }] },
    render: (row) => <StatusBadge status="completed" label={row.statusLabel} size="sm" />,
  },
  {
    key: "createdAt",
    header: "创建时间",
    width: 180,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.createdAt,
    filterValue: (row) => row.createdAt,
    copyValue: (row) => formatDateTime(row.createdAt),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.createdAt),
  },
  {
    key: "publishedAt",
    header: "发布时间",
    width: 180,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.publishedAt,
    filterValue: (row) => row.publishedAt,
    copyValue: (row) => formatDateTime(row.publishedAt),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.publishedAt),
  },
  {
    key: "publishedBy",
    header: "发布人",
    width: 260,
    minWidth: 180,
    mono: true,
    sortable: true,
    sortValue: (row) => row.publishedBy,
    filterValue: (row) => row.publishedBy,
    copyValue: (row) => row.publishedBy,
    filter: { type: "text" },
  },
];

type Notice = { type: "success" | "error"; text: string } | null;

export function H9PrintTemplatePage() {
  const librariesQuery = usePrintFieldLibrariesQuery();
  const templateTypesQuery = usePrintTemplateTypesQuery();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultH9QueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultH9QueryValue());
  const [selectedTreeNodeId, setSelectedTreeNodeId] = React.useState("");
  const [notice, setNotice] = React.useState<Notice>(null);
  const treeNodes = React.useMemo(
    () => buildH9TreeNodes(templateTypesQuery.data ?? [], librariesQuery.data ?? []),
    [librariesQuery.data, templateTypesQuery.data],
  );
  const treeScopedRows = React.useMemo(
    () => filterRowsByTree(librariesQuery.data ?? [], templateTypesQuery.data ?? [], selectedTreeNodeId),
    [librariesQuery.data, selectedTreeNodeId, templateTypesQuery.data],
  );
  const rows = React.useMemo(
    () => filterRows(treeScopedRows, appliedQuery),
    [appliedQuery, treeScopedRows],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h9PrintTemplateQueryFields, appliedQuery),
    [appliedQuery],
  );
  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新打印模板树和字段库列表",
    disabled: librariesQuery.isFetching || templateTypesQuery.isFetching,
    onClick: () => {
      void refreshLibraries();
    },
  };

  async function refreshLibraries() {
    setNotice(null);
    const [typesResult, librariesResult] = await Promise.all([
      templateTypesQuery.refetch(),
      librariesQuery.refetch(),
    ]);
    const error = typesResult.error ?? librariesResult.error;
    setNotice(
      error
        ? { type: "error", text: error.message }
        : { type: "success", text: "打印模板树已刷新" },
    );
  }

  function resetQuery() {
    const defaults = defaultH9QueryValue();
    setDraftQuery(defaults);
    setAppliedQuery(defaults);
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H9 打印模板"
        subtitle="字段库、模板类型和 hiprint 模板设计入口"
      />
      <NoticePanel notice={notice} />

      <QueryPanel
        fields={h9PrintTemplateQueryFields}
        defaultVisibleFieldKeys={h9PrintTemplateCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={(next) => setDraftQuery(normalizeH9QueryValue(next))}
        onQuery={() => setAppliedQuery(normalizeH9QueryValue(draftQuery))}
        onReset={resetQuery}
        resetLabel="重置"
      />

      {(librariesQuery.error || templateTypesQuery.error) && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {templateTypesQuery.error?.message ?? librariesQuery.error?.message}
        </div>
      )}

      <div className="grid gap-4 lg:grid-cols-[22rem_minmax(0,1fr)]">
        <TreeCatalog
          title="模板树"
          searchPlaceholder="搜索模板类型、字段库"
          nodes={treeNodes}
          selectedNodeId={selectedTreeNodeId}
          onSelectedNodeIdChange={setSelectedTreeNodeId}
          storageKey="h9.print-template.tree"
          emptyTitle={templateTypesQuery.isError ? "读取模板类型失败" : "暂无模板类型"}
          emptyDescription={
            templateTypesQuery.isError
              ? "请检查 print_template_type 字典和账号权限。"
              : "维护打印模板类型后将在这里显示。"
          }
        />
        <div className="min-w-0">
          <DataGrid
            storageKey="h9.print-template.field-libraries"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            caption={librariesQuery.isPending || templateTypesQuery.isPending ? "加载打印模板..." : undefined}
            emptyTitle={librariesQuery.isError ? "读取打印字段库失败" : "暂无匹配字段库"}
            emptyDescription={
              librariesQuery.isError
                ? "请检查后端 H9 接口和账号权限"
                : "请检查左侧模板类型绑定的字段库编码，或调整查询条件。"
            }
            exportFileBaseName="H9 打印模板字段库"
            refreshAction={refreshAction}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={(queryState) => {
              const next = normalizeH9QueryValue(queryValueFromUnknown(queryState));
              setDraftQuery(next);
              setAppliedQuery(next);
            }}
            onClearQueryState={resetQuery}
          />
        </div>
      </div>
    </section>
  );
}

function defaultH9QueryValue(): QueryPanelValue {
  return {
    keyword: "",
    sourceSchema: "",
  };
}

function normalizeH9QueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    sourceSchema: queryString(value.sourceSchema),
  };
}

function filterRows(rows: PrintFieldLibraryRow[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const sourceSchema = queryString(query.sourceSchema).trim().toLowerCase();
  return rows.filter((row) => {
    const sourceMatches = !sourceSchema || row.sourceSchema.toLowerCase().includes(sourceSchema);
    const keywordMatches = !keyword || row.searchText.includes(keyword);
    return sourceMatches && keywordMatches;
  });
}

function buildH9TreeNodes(
  templateTypes: PrintTemplateTypeRow[],
  libraries: PrintFieldLibraryRow[],
): TreeCatalogNode[] {
  const libraryByCode = new Map(libraries.map((library) => [library.libraryCode, library]));
  return templateTypes.map((type) => {
    const library = libraryByCode.get(type.fieldLibraryCode);
    const libraryNode: TreeCatalogNode = library
      ? {
          id: h9LibraryNodeId(library.libraryCode),
          label: library.libraryName,
          description: library.libraryCode,
          badge: `${library.fieldCount} 字段`,
          children: [
            {
              id: h9VersionNodeId(library.latestVersionId),
              label: `v${library.versionNo}`,
              description: formatDateTime(library.publishedAt),
              badge: "已发布",
            },
          ],
        }
      : {
          id: `missing:${type.code}`,
          label: "未发布字段库",
          description: type.fieldLibraryCode || "未绑定字段库编码",
          badge: "缺失",
          disabled: true,
        };
    return {
      id: h9TypeNodeId(type.code),
      label: type.name,
      description: type.code,
      badge: type.businessModule || "H9",
      disabled: !type.enabled,
      children: [libraryNode],
    };
  });
}

function filterRowsByTree(
  rows: PrintFieldLibraryRow[],
  templateTypes: PrintTemplateTypeRow[],
  selectedNodeId: string,
) {
  if (!selectedNodeId) return rows;
  if (selectedNodeId.startsWith("type:")) {
    const typeCode = selectedNodeId.slice("type:".length);
    const templateType = templateTypes.find((type) => type.code === typeCode);
    return templateType ? rows.filter((row) => row.libraryCode === templateType.fieldLibraryCode) : rows;
  }
  if (selectedNodeId.startsWith("library:")) {
    const libraryCode = selectedNodeId.slice("library:".length);
    return rows.filter((row) => row.libraryCode === libraryCode);
  }
  if (selectedNodeId.startsWith("version:")) {
    const versionId = selectedNodeId.slice("version:".length);
    return rows.filter((row) => row.latestVersionId === versionId);
  }
  return rows;
}

function h9TypeNodeId(code: string) {
  return `type:${code}`;
}

function h9LibraryNodeId(code: string) {
  return `library:${code}`;
}

function h9VersionNodeId(id: string) {
  return `version:${id}`;
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function queryValueFromUnknown(value: unknown): QueryPanelValue {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as QueryPanelValue) : {};
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  const success = notice.type === "success";
  return (
    <div
      className={cn(
        "rounded-md border px-3 py-2 text-sm",
        success
          ? "border-wms-success/30 bg-wms-success/10 text-wms-success"
          : "border-destructive/30 bg-destructive/10 text-destructive",
      )}
      role={success ? "status" : "alert"}
    >
      {notice.text}
    </div>
  );
}
