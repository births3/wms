import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  PageHeader,
  QueryPanel,
  StatusBadge,
  TreeCatalog,
  buildQueryPanelSummaryItems,
  cn,
  type DataGridColumn,
  type DataGridDisableAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
  type QueryPanelField,
  type QueryPanelValue,
  type TreeCatalogNode,
} from "@wms/ui";
import { Copy, Database, Eye, History, Upload } from "lucide-react";

import type { CurrentUser } from "@/features/auth/auth-queries";
import {
  usePrintFieldLibrariesQuery,
  usePublishPrintTemplateMutation,
  usePrintTemplateVersionsMutation,
  usePrintTemplatesQuery,
  usePrintTemplateTypesQuery,
  usePreviewPrintTemplateMutation,
  useRecordPrintTemplateMutation,
  useSavePrintTemplateMutation,
  useSetPrintTemplateEnabledMutation,
  type PrintFieldLibraryRow,
  type PrintTemplatePreviewResponse,
  type PrintTemplateRow,
  type PrintTemplateTypeRow,
  type PrintTemplateVersion,
  type SavePrintTemplateRequest,
} from "@/features/print-template/print-template-queries";

import { H9TemplateDesignerDialog, type H9TemplateDesignerMode } from "./H9TemplateDesignerDialog";
import { H9FieldLibraryDialog } from "./H9FieldLibraryDialog";
import { H9TemplatePreviewDialog } from "./H9TemplatePreviewDialog";

const h9PrintTemplateQueryFields: QueryPanelField[] = [
  {
    key: "keyword",
    label: "关键字",
    type: "text",
    placeholder: "搜索模板编码、名称或状态",
    ariaLabel: "搜索打印模板",
  },
  {
    key: "templateType",
    label: "模板类型",
    type: "text",
    placeholder: "例如 m2_asn",
    ariaLabel: "搜索模板类型",
  },
];
const h9PrintTemplateCoreQueryFieldKeys = ["keyword", "templateType"];

const columns: DataGridColumn<PrintTemplateRow>[] = [
  {
    key: "templateCode",
    header: "模板编码",
    width: 220,
    minWidth: 160,
    mono: true,
    sortable: true,
    sortValue: (row) => row.templateCode,
    filterValue: (row) => row.templateCode,
    copyValue: (row) => row.templateCode,
    filter: { type: "text" },
  },
  {
    key: "templateName",
    header: "模板名称",
    width: 220,
    minWidth: 160,
    sortable: true,
    sortValue: (row) => row.templateName,
    filterValue: (row) => row.templateName,
    copyValue: (row) => row.templateName,
    filter: { type: "text" },
  },
  {
    key: "templateTypeCode",
    header: "模板类型",
    width: 180,
    minWidth: 140,
    mono: true,
    sortable: true,
    sortValue: (row) => row.templateTypeCode,
    filterValue: (row) => row.templateTypeCode,
    copyValue: (row) => row.templateTypeCode,
    filter: { type: "text" },
  },
  {
    key: "latestVersionNo",
    header: "最新版本",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.latestVersionNo,
    filterValue: (row) => row.latestVersionNo,
    copyValue: (row) => `v${row.latestVersionNo}`,
    filter: { type: "numberRange" },
    render: (row) => `v${row.latestVersionNo}`,
  },
  {
    key: "latestVersionStatus",
    header: "版本状态",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.latestVersionStatus,
    filterValue: (row) => row.latestVersionStatus,
    copyValue: (row) => row.latestVersionStatus === "published" ? "已发布" : "草稿",
    filter: { type: "multiSelect", options: [{ label: "已发布", value: "published" }, { label: "草稿", value: "draft" }] },
    render: (row) => (
      <StatusBadge
        status={row.latestVersionStatus === "published" ? "completed" : "pending"}
        label={row.latestVersionStatus === "published" ? "已发布" : "草稿"}
        size="sm"
      />
    ),
  },
  {
    key: "scope",
    header: "作用域",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.scopeLabel,
    filterValue: (row) => row.scope,
    copyValue: (row) => row.scopeLabel,
    filter: { type: "multiSelect", options: [{ label: "全局", value: "global" }, { label: "货主", value: "owner" }] },
    render: (row) => row.scopeLabel,
  },
  {
    key: "status",
    header: "状态",
    width: 120,
    minWidth: 100,
    sortable: true,
    sortValue: (row) => row.statusLabel,
    filterValue: (row) => row.enabled ? "enabled" : "disabled",
    copyValue: (row) => row.statusLabel,
    filter: { type: "multiSelect", options: [{ label: "启用", value: "enabled" }, { label: "停用", value: "disabled" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "isolated"} label={row.statusLabel} size="sm" />,
  },
  {
    key: "isDefault",
    header: "默认",
    width: 100,
    minWidth: 80,
    sortable: true,
    sortValue: (row) => row.isDefault ? 1 : 0,
    filterValue: (row) => row.isDefault ? "yes" : "no",
    copyValue: (row) => row.isDefault ? "是" : "否",
    filter: { type: "multiSelect", options: [{ label: "是", value: "yes" }, { label: "否", value: "no" }] },
    render: (row) => row.isDefault ? "是" : "否",
  },
  {
    key: "designerVersion",
    header: "设计器",
    width: 160,
    minWidth: 120,
    sortable: true,
    sortValue: (row) => row.designerVersion,
    filterValue: (row) => row.designerVersion,
    copyValue: (row) => row.designerVersion,
    filter: { type: "text" },
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
    key: "updatedAt",
    header: "更新时间",
    width: 180,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.updatedAt,
    filterValue: (row) => row.updatedAt,
    copyValue: (row) => formatDateTime(row.updatedAt),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.updatedAt),
  },
  {
    key: "publishedAt",
    header: "发布时间",
    width: 180,
    minWidth: 140,
    sortable: true,
    sortValue: (row) => row.publishedAt ?? "",
    filterValue: (row) => row.publishedAt ?? "",
    copyValue: (row) => row.publishedAt ? formatDateTime(row.publishedAt) : "",
    filter: { type: "text" },
    render: (row) => row.publishedAt ? formatDateTime(row.publishedAt) : "-",
  },
];

type Notice = { type: "success" | "error"; text: string } | null;

export function H9PrintTemplatePage({ currentUser }: { currentUser: CurrentUser }) {
  const librariesQuery = usePrintFieldLibrariesQuery();
  const templatesQuery = usePrintTemplatesQuery();
  const templateTypesQuery = usePrintTemplateTypesQuery();
  const saveMutation = useSavePrintTemplateMutation();
  const publishMutation = usePublishPrintTemplateMutation();
  const enabledMutation = useSetPrintTemplateEnabledMutation();
  const versionsMutation = usePrintTemplateVersionsMutation();
  const previewMutation = usePreviewPrintTemplateMutation();
  const printMutation = useRecordPrintTemplateMutation();
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(() => defaultH9QueryValue());
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(() => defaultH9QueryValue());
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [selectedTreeNodeId, setSelectedTreeNodeId] = React.useState("");
  const [designerOpen, setDesignerOpen] = React.useState(false);
  const [designerMode, setDesignerMode] = React.useState<H9TemplateDesignerMode>("create");
  const [designerTemplate, setDesignerTemplate] = React.useState<PrintTemplateVersion | null>(null);
  const [previewOpen, setPreviewOpen] = React.useState(false);
  const [preview, setPreview] = React.useState<PrintTemplatePreviewResponse | null>(null);
  const [historyOpen, setHistoryOpen] = React.useState(false);
  const [fieldLibraryOpen, setFieldLibraryOpen] = React.useState(false);
  const [historyVersions, setHistoryVersions] = React.useState<PrintTemplateVersion[]>([]);
  const [notice, setNotice] = React.useState<Notice>(null);
  const canWriteTemplate = currentUser.permissions.includes("h9.print_template.write");
  const canPublishTemplate = currentUser.permissions.includes("h9.print_template.publish");
  const canPrintTemplate = currentUser.permissions.includes("h9.print_template.print");
  const canMaintainFieldLibrary = canWriteTemplate;
  const canPublishFieldLibrary = canPublishTemplate;
  const canOpenFieldLibrary = canMaintainFieldLibrary || canPublishFieldLibrary;
  const treeNodes = React.useMemo(
    () => buildH9TreeNodes(templateTypesQuery.data ?? [], librariesQuery.data ?? []),
    [librariesQuery.data, templateTypesQuery.data],
  );
  const treeScopedRows = React.useMemo(
    () => filterRowsByTree(templatesQuery.data ?? [], templateTypesQuery.data ?? [], selectedTreeNodeId),
    [selectedTreeNodeId, templateTypesQuery.data, templatesQuery.data],
  );
  const rows = React.useMemo(
    () => filterRows(treeScopedRows, appliedQuery),
    [appliedQuery, treeScopedRows],
  );
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(h9PrintTemplateQueryFields, appliedQuery),
    [appliedQuery],
  );
  const templateById = React.useMemo(() => new Map((templatesQuery.data ?? []).map((row) => [row.id, row])), [templatesQuery.data]);
  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新打印模板树和模板列表",
    disabled: librariesQuery.isFetching || templateTypesQuery.isFetching || templatesQuery.isFetching,
    onClick: () => {
      void refreshLibraries();
    },
  };
  const createAction = {
    label: "新增",
    description: "打开 hiprint 打印模板设计器",
    disabled: librariesQuery.isPending || templateTypesQuery.isPending,
    onClick: () => openCreateDesigner(),
  };
  const editAction: DataGridEditAction = {
    label: "修改",
    description: "修改选中模板并保存新版本",
    disabled: (context) => context.selectedRowKeys.length !== 1 || versionsMutation.isPending,
    onClick: (context) => void openDesignerFromRow(context.selectedRowKeys[0], "edit"),
  };
  const selectedRow = selectedTemplateRow(selectedRowKeys);
  const disableAction: DataGridDisableAction = {
    label: selectedRow?.enabled === false ? "启用" : "停用",
    description: selectedRow?.enabled === false ? "启用选中模板" : "停用选中模板",
    disabled: (context) => context.selectedRowKeys.length !== 1 || enabledMutation.isPending,
    onClick: (context) => void toggleTemplateEnabled(context.selectedRowKeys[0]),
  };
  const toolbarActions: DataGridToolbarAction[] = [];
  if (canWriteTemplate) {
    toolbarActions.push({
      key: "copy-template",
      label: "复制",
      description: "复制选中模板并生成副本",
      icon: <Copy className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || versionsMutation.isPending,
      onClick: (context) => void openDesignerFromRow(context.selectedRowKeys[0], "copy"),
    });
  }
  if (canPublishTemplate) {
    toolbarActions.push({
      key: "publish-template",
      label: "发布",
      description: "发布选中模板的最新草稿",
      icon: <Upload className="size-4" aria-hidden />,
      disabled: (context) =>
        context.selectedRowKeys.length !== 1
        || selectedTemplateRow(context.selectedRowKeys)?.latestVersionStatus !== "draft"
        || publishMutation.isPending,
      onClick: (context) => void publishTemplate(context.selectedRowKeys[0]),
    });
  }
  toolbarActions.push(
    {
      key: "version-history",
      label: "版本",
      description: "查看版本历史",
      icon: <History className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1 || versionsMutation.isPending,
      onClick: (context) => void openVersionHistory(context.selectedRowKeys[0]),
    },
    {
      key: "preview-template",
      label: "预览",
      description: "按选中模板生成浏览器预览",
      icon: <Eye className="size-4" aria-hidden />,
      disabled: (context) => context.selectedRowKeys.length !== 1,
      onClick: (context) => void previewTemplate(context.selectedRowKeys[0]),
    },
  );

  async function refreshLibraries() {
    setNotice(null);
    const [typesResult, librariesResult, templatesResult] = await Promise.all([
      templateTypesQuery.refetch(),
      librariesQuery.refetch(),
      templatesQuery.refetch(),
    ]);
    const error = typesResult.error ?? librariesResult.error ?? templatesResult.error;
    setNotice(
      error
        ? { type: "error", text: error.message }
        : { type: "success", text: "打印模板已刷新" },
    );
  }

  function resetQuery() {
    const defaults = defaultH9QueryValue();
    setDraftQuery(defaults);
    setAppliedQuery(defaults);
  }

  async function saveTemplate(request: SavePrintTemplateRequest) {
    const saved = await saveMutation.mutateAsync(request);
    setNotice({ type: "success", text: `${saved.template_code} 草稿已保存` });
  }

  function selectedTemplateRow(keys: string[]) {
    return keys.length === 1 ? templateById.get(keys[0]) ?? null : null;
  }

  function openCreateDesigner() {
    setDesignerMode("create");
    setDesignerTemplate(null);
    setDesignerOpen(true);
  }

  async function openDesignerFromRow(rowId: string, mode: H9TemplateDesignerMode) {
    try {
      const latest = await latestTemplateVersion(rowId);
      setDesignerMode(mode);
      setDesignerTemplate(latest);
      setDesignerOpen(true);
    } catch (errorValue) {
      setNotice({ type: "error", text: errorValue instanceof Error ? errorValue.message : "读取模板版本失败" });
    }
  }

  async function toggleTemplateEnabled(rowId: string) {
    try {
      const row = templateById.get(rowId);
      if (!row) return;
      if (!window.confirm(`确认${row.enabled ? "停用" : "启用"}模板「${row.templateName}」？`)) return;
      const saved = await enabledMutation.mutateAsync({
        templateId: row.id,
        body: { enabled: !row.enabled },
      });
      setNotice({ type: "success", text: `${saved.template_code} 已${saved.enabled ? "启用" : "停用"}` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorValue instanceof Error ? errorValue.message : "停启模板失败" });
    }
  }

  async function publishTemplate(rowId: string) {
    const row = templateById.get(rowId);
    if (!row || row.latestVersionStatus !== "draft") return;
    if (!window.confirm(`确认发布模板「${row.templateName}」的 v${row.latestVersionNo} 草稿？`)) return;
    try {
      const published = await publishMutation.mutateAsync({
        templateId: row.id,
        versionId: row.latestVersionId,
      });
      setNotice({ type: "success", text: `${published.template_code} v${published.version_no} 已发布` });
    } catch (errorValue) {
      setNotice({ type: "error", text: errorValue instanceof Error ? errorValue.message : "发布模板失败" });
    }
  }

  async function openVersionHistory(rowId: string) {
    try {
      const row = templateById.get(rowId);
      if (!row) return;
      const versions = await versionsMutation.mutateAsync(row.id);
      setHistoryVersions(versions);
      setHistoryOpen(true);
    } catch (errorValue) {
      setNotice({ type: "error", text: errorValue instanceof Error ? errorValue.message : "读取版本历史失败" });
    }
  }

  async function latestTemplateVersion(rowId: string) {
    const row = templateById.get(rowId);
    if (!row) throw new Error("未选中打印模板");
    const versions = await versionsMutation.mutateAsync(row.id);
    const latest = versions[0];
    if (!latest) throw new Error("打印模板没有版本");
    return latest;
  }

  async function previewTemplate(rowId: string) {
    const row = templateById.get(rowId);
    if (!row) return;
    if (!window.confirm(`确认生成模板「${row.templateName}」的打印预览？`)) return;
    try {
      const next = await previewMutation.mutateAsync({
        template_code: row.templateCode,
        template_type_code: row.templateTypeCode,
        business_document_id: "H9-SAMPLE",
        data: samplePrintData(),
      });
      setPreview(next);
      setPreviewOpen(true);
    } catch (errorValue) {
      setNotice({ type: "error", text: errorValue instanceof Error ? errorValue.message : "预览失败" });
    }
  }

  async function recordPrint() {
    if (!preview) return;
    await printMutation.mutateAsync({
      template_code: preview.template_code,
      template_type_code: preview.template_type_code,
      business_module: preview.template_type_code.split("_")[0]?.toUpperCase() || "H9",
      business_document_type: preview.template_type_code,
      business_document_id: "H9-SAMPLE",
      data: preview.data,
      status: "printed",
      failure_reason: null,
    });
    setNotice({ type: "success", text: "打印记录已写入" });
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        title="H9 打印模板"
        subtitle="字段库、模板类型和 hiprint 模板设计入口"
        actions={canOpenFieldLibrary ? (
          <Button type="button" variant="outline" onClick={() => setFieldLibraryOpen(true)}>
            <Database className="size-4" aria-hidden />
            字段库管理
          </Button>
        ) : null}
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

      {(librariesQuery.error || templateTypesQuery.error || templatesQuery.error) && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {templateTypesQuery.error?.message ?? librariesQuery.error?.message ?? templatesQuery.error?.message}
        </div>
      )}

      <div className="grid gap-4 lg:grid-cols-[22rem_minmax(0,1fr)]">
        <TreeCatalog
          title="模板树"
          searchable={false}
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
            storageKey="h9.print-template.templates"
            columns={columns}
            data={rows}
            rowKey={(row) => row.id}
            selectable
            selectedRowKeys={selectedRowKeys}
            onSelectedRowKeysChange={setSelectedRowKeys}
            caption={librariesQuery.isPending || templateTypesQuery.isPending || templatesQuery.isPending ? "加载打印模板..." : undefined}
            emptyTitle={templatesQuery.isError ? "读取打印模板失败" : "暂无匹配模板"}
            emptyDescription={
              templatesQuery.isError
                ? "请检查后端 H9 接口和账号权限"
                : "点击新增打开 hiprint 设计器，或调整查询条件。"
            }
            exportFileBaseName="H9 打印模板"
            refreshAction={refreshAction}
            createAction={canWriteTemplate ? createAction : undefined}
            editAction={canWriteTemplate ? editAction : undefined}
            disableAction={canWriteTemplate ? disableAction : undefined}
            printAction={canPrintTemplate ? {
              label: "打印",
              description: "预览并打印选中模板",
              disabled: (context) => context.selectedRowKeys.length !== 1,
              onClick: (context) => void previewTemplate(context.selectedRowKeys[0]),
            } : undefined}
            toolbarActions={toolbarActions}
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
      <H9TemplateDesignerDialog
        open={designerOpen}
        mode={designerMode}
        initialTemplate={designerTemplate}
        templateTypes={templateTypesQuery.data ?? []}
        libraries={librariesQuery.data ?? []}
        onOpenChange={(open) => {
          setDesignerOpen(open);
          if (!open) setDesignerTemplate(null);
        }}
        onSave={saveTemplate}
      />
      <H9FieldLibraryDialog
        open={fieldLibraryOpen}
        libraries={librariesQuery.data ?? []}
        canMaintain={canMaintainFieldLibrary}
        canPublish={canPublishFieldLibrary}
        onOpenChange={setFieldLibraryOpen}
      />
      <VersionHistoryDialog open={historyOpen} versions={historyVersions} onOpenChange={setHistoryOpen} />
      <H9TemplatePreviewDialog
        open={previewOpen}
        preview={preview}
        onOpenChange={setPreviewOpen}
        onPrint={recordPrint}
      />
    </section>
  );
}

function defaultH9QueryValue(): QueryPanelValue {
  return {
    keyword: "",
    templateType: "",
  };
}

function normalizeH9QueryValue(value: QueryPanelValue): QueryPanelValue {
  return {
    keyword: queryString(value.keyword),
    templateType: queryString(value.templateType),
  };
}

function filterRows(rows: PrintTemplateRow[], query: QueryPanelValue) {
  const keyword = queryString(query.keyword).trim().toLowerCase();
  const templateType = queryString(query.templateType).trim().toLowerCase();
  return rows.filter((row) => {
    const sourceMatches = !templateType || row.templateTypeCode.toLowerCase().includes(templateType);
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
    const libraryNode: TreeCatalogNode = library?.publishedVersionId
      ? {
          id: h9LibraryNodeId(library.libraryCode),
          label: library.libraryName,
          description: library.libraryCode,
          badge: `${library.fieldCount} 字段`,
          children: [
            {
              id: h9VersionNodeId(library.publishedVersionId),
              label: `v${library.publishedVersionNo}`,
              description: library.sourceSchema,
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
  rows: PrintTemplateRow[],
  templateTypes: PrintTemplateTypeRow[],
  selectedNodeId: string,
) {
  if (!selectedNodeId) return rows;
  if (selectedNodeId.startsWith("type:")) {
    const typeCode = selectedNodeId.slice("type:".length);
    return rows.filter((row) => row.templateTypeCode === typeCode);
  }
  if (selectedNodeId.startsWith("library:")) {
    const libraryCode = selectedNodeId.slice("library:".length);
    const typeCodes = templateTypes.filter((type) => type.fieldLibraryCode === libraryCode).map((type) => type.code);
    return rows.filter((row) => typeCodes.includes(row.templateTypeCode));
  }
  if (selectedNodeId.startsWith("version:")) {
    const versionId = selectedNodeId.slice("version:".length);
    const typeCodes = templateTypes
      .filter((type) => rows.some((row) => row.fieldLibraryVersionId === versionId && row.templateTypeCode === type.code))
      .map((type) => type.code);
    return rows.filter((row) => row.fieldLibraryVersionId === versionId || typeCodes.includes(row.templateTypeCode));
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

function VersionHistoryDialog({
  open,
  versions,
  onOpenChange,
}: {
  open: boolean;
  versions: PrintTemplateVersion[];
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <DialogHeader>
          <DialogTitle>版本历史</DialogTitle>
          <DialogDescription>按版本号倒序展示当前模板的保存记录。</DialogDescription>
        </DialogHeader>
        <div className="overflow-hidden rounded-md border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-muted-foreground">
              <tr>
                <th className="px-3 py-2 text-left font-medium">版本</th>
                <th className="px-3 py-2 text-left font-medium">状态</th>
                <th className="px-3 py-2 text-left font-medium">设计器</th>
                <th className="px-3 py-2 text-left font-medium">创建时间</th>
              </tr>
            </thead>
            <tbody>
              {versions.map((version) => (
                <tr key={version.id} className="border-t">
                  <td className="px-3 py-2">v{version.version_no}</td>
                  <td className="px-3 py-2">{version.status === "published" ? "已发布" : "草稿"}</td>
                  <td className="px-3 py-2">{version.designer_version}</td>
                  <td className="px-3 py-2">{formatDateTime(version.created_at)}</td>
                </tr>
              ))}
              {versions.length === 0 && (
                <tr>
                  <td className="px-3 py-6 text-center text-muted-foreground" colSpan={4}>
                    暂无版本记录
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="flex justify-end">
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
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

function samplePrintData() {
  return {
    asn: { code: "ASN-202607070001" },
    inspection: { code: "INS-202607070001" },
    outbound: { delivery_no: "DN-202607070001" },
    location: { code: "A01-01-02-03" },
    lpn: { code: "LPN-202607070001" },
    product: { code: "P-M1-001", name: "冷藏胰岛素注射液" },
  };
}
