import * as React from "react";
import {
  Button,
  Card,
  CardContent,
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
  buildQueryPanelSummaryItems,
  cn,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridDetailAction,
  type DataGridDisableAction,
  type DataGridEditAction,
  type DataGridRefreshAction,
  type QueryPanelField,
  type QueryPanelValue,
} from "@wms/ui";
import { History } from "lucide-react";

import {
  useDocumentNumberAllocationsQuery,
  useDocumentNumberRulesQuery,
  useSetDocumentNumberRuleEnabledMutation,
  useUpsertDocumentNumberRuleMutation,
  type DocumentNumberAllocation,
  type DocumentNumberRule,
  type UpsertDocumentNumberRuleRequest,
} from "@/features/document-numbering/document-numbering-queries";
import { useSystemDictionaryItemOptionsQuery } from "@/features/master-data/master-data-queries";

type DocumentTypeOption = { label: string; value: string };

export function buildMcgDocumentNumberQueryFields(
  documentTypeOptions: DocumentTypeOption[],
): QueryPanelField[] {
  return [{
    key: "documentType",
    label: "单据类型",
    type: "select",
    options: [{ label: "全部", value: "" }, ...documentTypeOptions],
  }];
}
export const mcgDocumentNumberCoreQueryFieldKeys = ["documentType"];

function buildRuleColumns(documentTypeOptions: DocumentTypeOption[]): DataGridColumn<DocumentNumberRule>[] {
  return [
  textColumn<DocumentNumberRule>("rule_code", "规则编码", 180),
  {
    key: "document_type",
    header: "单据类型",
    width: 150,
    sortable: true,
    sortValue: (row) => documentTypeLabel(row.document_type, documentTypeOptions),
    filterValue: (row) => row.document_type,
    copyValue: (row) => row.document_type,
    filter: { type: "multiSelect", options: documentTypeOptions },
    render: (row) => documentTypeLabel(row.document_type, documentTypeOptions),
  },
  textColumn<DocumentNumberRule>("rule_name", "规则名称", 190),
  {
    key: "template",
    header: "模板",
    width: 280,
    minWidth: 220,
    copyValue: (row) => row.template,
    filterValue: (row) => row.template,
    filter: { type: "text" },
    render: (row) => <code className="text-xs text-muted-foreground">{row.template}</code>,
  },
  {
    key: "reset_policy",
    header: "重置策略",
    width: 120,
    sortable: true,
    sortValue: (row) => row.reset_policy,
    filterValue: (row) => row.reset_policy,
    copyValue: (row) => row.reset_policy,
    filter: { type: "multiSelect", options: [{ label: "每日", value: "daily" }, { label: "连续", value: "continuous" }] },
    render: (row) => row.reset_policy === "daily" ? "每日" : "连续",
  },
  {
    key: "sequence_width",
    header: "流水位数",
    width: 110,
    sortable: true,
    sortValue: (row) => row.sequence_width,
    filterValue: (row) => String(row.sequence_width),
    copyValue: (row) => String(row.sequence_width),
    filter: { type: "text" },
    render: (row) => `${row.sequence_width} 位`,
  },
  {
    key: "enabled",
    header: "状态",
    width: 100,
    sortable: true,
    sortValue: (row) => String(row.enabled),
    filterValue: (row) => row.enabled ? "enabled" : "disabled",
    copyValue: (row) => row.enabled ? "启用" : "停用",
    filter: { type: "multiSelect", options: [{ label: "启用", value: "enabled" }, { label: "停用", value: "disabled" }] },
    render: (row) => <StatusBadge status={row.enabled ? "completed" : "offline_cached"} label={row.enabled ? "启用" : "停用"} size="sm" />,
  },
  {
    key: "updated_at",
    header: "更新时间",
    width: 175,
    sortable: true,
    sortValue: (row) => row.updated_at,
    filterValue: (row) => row.updated_at,
    copyValue: (row) => formatDateTime(row.updated_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.updated_at),
  },
  {
    key: "created_at",
    header: "创建时间",
    width: 175,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.created_at),
  },
  ];
}

function buildAllocationColumns(
  documentTypeOptions: DocumentTypeOption[],
): DataGridColumn<DocumentNumberAllocation>[] {
  return [
  textColumn<DocumentNumberAllocation>("generated_no", "生成单号", 230),
  {
    key: "document_type",
    header: "单据类型",
    width: 150,
    filterValue: (row) => row.document_type,
    copyValue: (row) => row.document_type,
    filter: { type: "multiSelect", options: documentTypeOptions },
    render: (row) => documentTypeLabel(row.document_type, documentTypeOptions),
  },
  textColumn<DocumentNumberAllocation>("source_module", "来源模块", 140),
  {
    key: "sequence_value",
    header: "流水值",
    width: 100,
    sortable: true,
    sortValue: (row) => row.sequence_value,
    filterValue: (row) => String(row.sequence_value),
    copyValue: (row) => String(row.sequence_value),
    filter: { type: "text" },
  },
  textColumn<DocumentNumberAllocation>("counter_key", "计数器键", 220),
  {
    key: "created_at",
    header: "生成时间",
    width: 175,
    sortable: true,
    sortValue: (row) => row.created_at,
    filterValue: (row) => row.created_at,
    copyValue: (row) => formatDateTime(row.created_at),
    filter: { type: "text" },
    render: (row) => formatDateTime(row.created_at),
  },
  ];
}

type Notice = { type: "success" | "error"; text: string } | null;
type RuleForm = {
  ruleCode: string;
  documentType: string;
  ruleName: string;
  template: string;
  resetPolicy: string;
  sequenceWidth: string;
  enabled: boolean;
  effectiveFrom: string;
  effectiveTo: string;
};

export function MCGDocumentNumberingPage() {
  const [draftQuery, setDraftQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const [appliedQuery, setAppliedQuery] = React.useState<QueryPanelValue>(defaultQuery);
  const [selectedRowKeys, setSelectedRowKeys] = React.useState<string[]>([]);
  const [editorOpen, setEditorOpen] = React.useState(false);
  const [disableDialogOpen, setDisableDialogOpen] = React.useState(false);
  const [previewOpen, setPreviewOpen] = React.useState(false);
  const [editingRule, setEditingRule] = React.useState<DocumentNumberRule | null>(null);
  const [disableRuleTarget, setDisableRuleTarget] = React.useState<DocumentNumberRule | null>(null);
  const [previewRule, setPreviewRule] = React.useState<DocumentNumberRule | null>(null);
  const [form, setForm] = React.useState<RuleForm>(defaultForm);
  const [notice, setNotice] = React.useState<Notice>(null);
  const documentTypeOptionsQuery = useSystemDictionaryItemOptionsQuery("document_type");
  const documentTypeOptions = React.useMemo(
    () => (documentTypeOptionsQuery.data ?? []).map(([value, label]) => ({ value, label })),
    [documentTypeOptionsQuery.data],
  );
  const mcgDocumentNumberQueryFields = React.useMemo(
    () => buildMcgDocumentNumberQueryFields(documentTypeOptions),
    [documentTypeOptions],
  );
  const ruleColumns = React.useMemo(() => buildRuleColumns(documentTypeOptions), [documentTypeOptions]);
  const allocationColumns = React.useMemo(
    () => buildAllocationColumns(documentTypeOptions),
    [documentTypeOptions],
  );
  const documentType = queryString(appliedQuery.documentType);
  const rulesQuery = useDocumentNumberRulesQuery(documentType);
  const allocationsQuery = useDocumentNumberAllocationsQuery(documentType);
  const upsertMutation = useUpsertDocumentNumberRuleMutation();
  const enabledMutation = useSetDocumentNumberRuleEnabledMutation();
  const rules = rulesQuery.data ?? [];
  const selectedRule = rules.find((row) => row.rule_code === selectedRowKeys[0]);
  const busy = upsertMutation.isPending || enabledMutation.isPending;
  const dictionaryReady = !documentTypeOptionsQuery.isPending && !documentTypeOptionsQuery.isError;

  const refreshAction: DataGridRefreshAction = {
    label: "刷新",
    description: "刷新编码规则和生成记录",
    disabled: rulesQuery.isFetching || allocationsQuery.isFetching,
    onClick: () => void refresh(),
  };
  const createAction: DataGridCreateAction = {
    label: "新增",
    description: "新增单据号生成规则",
    disabled: busy || !dictionaryReady,
    onClick: () => openEditor(null),
  };
  const editAction: DataGridEditAction = {
    label: "编辑",
    description: "编辑选中的单据号规则",
    disabled: (context) => context.selectedRowKeys.length !== 1 || busy,
    onClick: (context) => openEditor(rules.find((row) => row.rule_code === context.selectedRowKeys[0]) ?? null),
  };
  const detailAction: DataGridDetailAction = {
    label: "预览",
    description: "预览选中规则生成的示例单号",
    disabled: (context) => context.selectedRowKeys.length !== 1,
    onClick: (context) => {
      const rule = rules.find((row) => row.rule_code === context.selectedRowKeys[0]);
      if (rule) {
        setPreviewRule(rule);
        setPreviewOpen(true);
      }
    },
  };
  const disableAction: DataGridDisableAction = {
    label: "启停",
    description: "启用或停用选中的单据号规则",
    disabled: (context) => context.selectedRowKeys.length !== 1 || busy,
    onClick: (context) => {
      const rule = rules.find((row) => row.rule_code === context.selectedRowKeys[0]);
      if (rule) {
        setDisableRuleTarget(rule);
        setDisableDialogOpen(true);
      }
    },
  };
  const querySummaryItems = React.useMemo(
    () => buildQueryPanelSummaryItems(mcgDocumentNumberQueryFields, appliedQuery),
    [appliedQuery, mcgDocumentNumberQueryFields],
  );

  async function refresh() {
    const [rulesResult, allocationResult] = await Promise.all([rulesQuery.refetch(), allocationsQuery.refetch()]);
    setNotice(rulesResult.error || allocationResult.error
      ? { type: "error", text: errorMessage(rulesResult.error ?? allocationResult.error, "刷新编码规则失败") }
      : { type: "success", text: "编码规则和生成记录已刷新" });
  }

  function openEditor(rule: DocumentNumberRule | null) {
    setNotice(null);
    setEditingRule(rule);
    setForm(rule ? formFromRule(rule) : defaultForm());
    setEditorOpen(true);
  }

  async function toggleRule(rule?: DocumentNumberRule) {
    if (!rule) return;
    setNotice(null);
    try {
      await enabledMutation.mutateAsync({ ruleCode: rule.rule_code, enabled: !rule.enabled });
      setSelectedRowKeys([]);
      setNotice({ type: "success", text: `${rule.rule_name} 已${rule.enabled ? "停用" : "启用"}` });
    } catch (error) {
      setNotice({ type: "error", text: errorMessage(error, "更新规则状态失败") });
    }
  }

  async function confirmToggleRule() {
    await toggleRule(disableRuleTarget ?? undefined);
    setDisableDialogOpen(false);
    setDisableRuleTarget(null);
  }

  async function submitForm(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const ruleCode = form.ruleCode.trim();
    const sequenceWidth = Number(form.sequenceWidth);
    if (!ruleCode || !form.ruleName.trim() || !form.template.trim() || !Number.isInteger(sequenceWidth) || sequenceWidth < 1 || sequenceWidth > 12) {
      setNotice({ type: "error", text: "请填写规则编码、名称、模板，并将流水位数设置为 1 至 12 的整数" });
      return;
    }
    const body: UpsertDocumentNumberRuleRequest = {
      document_type: form.documentType,
      rule_name: form.ruleName.trim(),
      template: form.template.trim(),
      reset_policy: form.resetPolicy,
      sequence_width: sequenceWidth,
      enabled: form.enabled,
      effective_from: form.effectiveFrom ? new Date(form.effectiveFrom).toISOString() : null,
      effective_to: form.effectiveTo ? new Date(form.effectiveTo).toISOString() : null,
    };
    try {
      await upsertMutation.mutateAsync({ ruleCode, body });
      setEditorOpen(false);
      setNotice({ type: "success", text: `${editingRule ? "规则已更新" : "规则已新增"}，历史单号保持不变` });
    } catch (error) {
      setNotice({ type: "error", text: errorMessage(error, "保存单据号规则失败") });
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader title="M-CG 单据号规则" subtitle="统一维护单据类型、模板、流水策略和生成记录" />
      <NoticePanel
        notice={notice ?? (documentTypeOptionsQuery.isError
          ? { type: "error", text: errorMessage(documentTypeOptionsQuery.error, "读取 M1 单据类型字典失败") }
          : null)}
      />
      <QueryPanel
        fields={mcgDocumentNumberQueryFields}
        defaultVisibleFieldKeys={mcgDocumentNumberCoreQueryFieldKeys}
        value={draftQuery}
        onValueChange={setDraftQuery}
        onQuery={() => setAppliedQuery(draftQuery)}
        onReset={() => {
          const next = defaultQuery();
          setDraftQuery(next);
          setAppliedQuery(next);
        }}
      />
      <Card className="rounded-lg shadow-sm">
        <CardContent className="p-5">
          <DataGrid
            storageKey="mcg.document-number-rules"
            columns={ruleColumns}
            data={rules}
            rowKey={(row) => row.rule_code}
            selectable
            selectedRowKeys={selectedRowKeys}
            onSelectedRowKeysChange={setSelectedRowKeys}
            caption={rulesQuery.isPending ? "加载编码规则..." : undefined}
            emptyTitle={rulesQuery.isError ? "读取编码规则失败" : "暂无编码规则"}
            emptyDescription={rulesQuery.isError ? errorMessage(rulesQuery.error, "请检查权限和数据库连接") : "请新增或导入受控的单据号规则"}
            exportFileBaseName="M-CG-单据号规则"
            refreshAction={refreshAction}
            createAction={createAction}
            detailAction={detailAction}
            editAction={editAction}
            disableAction={disableAction}
            queryState={appliedQuery}
            querySummaryItems={querySummaryItems}
            onApplyQueryState={(value) => {
              const next = normalizeQuery(value);
              setDraftQuery(next);
              setAppliedQuery(next);
            }}
            onClearQueryState={() => {
              const next = defaultQuery();
              setDraftQuery(next);
              setAppliedQuery(next);
            }}
          />
        </CardContent>
      </Card>
      <Card className="rounded-lg shadow-sm">
        <CardContent className="p-5">
          <div className="mb-4 flex items-center gap-2">
            <History className="size-4 text-primary" aria-hidden />
            <div>
              <h2 className="text-base font-semibold">生成记录</h2>
              <p className="text-xs text-muted-foreground">按当前单据类型查看实际分配结果，历史单号不可修改。</p>
            </div>
          </div>
          <DataGrid
            storageKey="mcg.document-number-allocations"
            columns={allocationColumns}
            data={allocationsQuery.data ?? []}
            rowKey={(row) => row.id}
            caption={allocationsQuery.isPending ? "加载生成记录..." : undefined}
            emptyTitle={allocationsQuery.isError ? "读取生成记录失败" : "暂无生成记录"}
            emptyDescription={allocationsQuery.isError ? errorMessage(allocationsQuery.error, "请检查权限和数据库连接") : "业务单据生成编号后会出现在这里"}
            exportFileBaseName="M-CG-单据号生成记录"
            refreshAction={refreshAction}
          />
        </CardContent>
      </Card>

      <Dialog open={editorOpen} onOpenChange={(open) => !busy && setEditorOpen(open)}>
        <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
          <form className="grid gap-4" onSubmit={submitForm}>
            <DialogHeader>
              <DialogTitle>{editingRule ? "编辑单据号规则" : "新增单据号规则"}</DialogTitle>
              <DialogDescription>单据类型必须来自 M1 系统字典；修改规则不会改变已生成的历史单号。</DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="grid gap-1 text-sm">规则编码<Input required disabled={Boolean(editingRule)} value={form.ruleCode} onChange={(event) => updateForm("ruleCode", event.target.value)} placeholder="purchase-inbound" /></label>
              <label className="grid gap-1 text-sm">规则名称<Input required value={form.ruleName} onChange={(event) => updateForm("ruleName", event.target.value)} placeholder="采购入库单号" /></label>
              <label className="grid gap-1 text-sm">单据类型<select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={form.documentType} onChange={(event) => updateForm("documentType", event.target.value)} aria-label="单据类型" disabled={!dictionaryReady}><option value="">请选择</option>{documentTypeOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}</select></label>
              <label className="grid gap-1 text-sm">重置策略<select className="h-9 rounded-md border border-input bg-background px-3 text-sm" value={form.resetPolicy} onChange={(event) => updateForm("resetPolicy", event.target.value)} aria-label="重置策略"><option value="daily">每日重置</option><option value="continuous">连续递增</option></select></label>
              <label className="grid gap-1 text-sm">流水位数<Input required type="number" min="1" max="12" step="1" value={form.sequenceWidth} onChange={(event) => updateForm("sequenceWidth", event.target.value)} /></label>
              <label className="flex items-center gap-2 self-end pb-2 text-sm"><input type="checkbox" checked={form.enabled} onChange={(event) => setForm((current) => ({ ...current, enabled: event.target.checked }))} />保存后启用</label>
            </div>
            <label className="grid gap-1 text-sm">编码模板<span className="text-xs text-muted-foreground">可用占位符：{`{OWNER}`}、{`{YYYY}`}、{`{YY}`}、{`{MM}`}、{`{DD}`}、{`{SEQ}`}</span><textarea required className="min-h-24 rounded-md border border-input bg-background px-3 py-2 font-mono text-sm" value={form.template} onChange={(event) => updateForm("template", event.target.value)} placeholder="{OWNER}-ASN-{YYYY}{MM}{DD}-{SEQ}" /></label>
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="grid gap-1 text-sm">生效时间<Input type="datetime-local" value={form.effectiveFrom} onChange={(event) => updateForm("effectiveFrom", event.target.value)} /></label>
              <label className="grid gap-1 text-sm">失效时间<Input type="datetime-local" value={form.effectiveTo} onChange={(event) => updateForm("effectiveTo", event.target.value)} /></label>
            </div>
            <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="submit" disabled={busy}>{busy ? "保存中..." : "保存"}</Button></DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      <Dialog open={disableDialogOpen} onOpenChange={(open) => !busy && setDisableDialogOpen(open)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader><DialogTitle>{disableRuleTarget?.enabled ? "停用编码规则" : "启用编码规则"}</DialogTitle><DialogDescription>停用只影响后续发号，不会修改已经生成的历史单号。</DialogDescription></DialogHeader>
          <p className="text-sm">确认{disableRuleTarget?.enabled ? "停用" : "启用"}规则“{disableRuleTarget?.rule_name ?? ""}”？</p>
          <DialogFooter><DialogClose asChild><Button type="button" variant="outline" disabled={busy}>取消</Button></DialogClose><Button type="button" disabled={busy} onClick={() => void confirmToggleRule()}>{busy ? "处理中..." : disableRuleTarget?.enabled ? "确认停用" : "确认启用"}</Button></DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={previewOpen} onOpenChange={setPreviewOpen}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader><DialogTitle>规则预览</DialogTitle><DialogDescription>预览只生成示例，不会占用真实流水号。</DialogDescription></DialogHeader>
          {previewRule ? <div className="grid gap-4 text-sm"><div className="rounded-md border bg-muted/30 p-4"><div className="mb-2 text-xs text-muted-foreground">示例单据号</div><code className="break-all text-base font-semibold text-primary">{renderPreview(previewRule.template, previewRule.sequence_width)}</code></div><dl className="grid grid-cols-[110px_1fr] gap-x-4 gap-y-2"><dt className="text-muted-foreground">规则编码</dt><dd>{previewRule.rule_code}</dd><dt className="text-muted-foreground">单据类型</dt><dd>{documentTypeLabel(previewRule.document_type, documentTypeOptions)}</dd><dt className="text-muted-foreground">流水策略</dt><dd>{previewRule.reset_policy === "daily" ? "每日重置" : "连续递增"} / {previewRule.sequence_width} 位</dd><dt className="text-muted-foreground">状态</dt><dd>{previewRule.enabled ? "启用" : "停用"}</dd></dl></div> : null}
          <DialogFooter><Button type="button" variant="outline" onClick={() => setPreviewOpen(false)}>关闭</Button></DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );

  function updateForm(key: keyof RuleForm, value: string) {
    setForm((current) => ({ ...current, [key]: value }));
  }
}

function textColumn<T extends object>(key: keyof T & string, header: string, width: number): DataGridColumn<T> {
  return {
    key: String(key),
    header,
    width,
    minWidth: Math.min(width, 120),
    sortable: true,
    sortValue: (row) => String(row[key] ?? ""),
    filterValue: (row) => String(row[key] ?? ""),
    copyValue: (row) => String(row[key] ?? ""),
    filter: { type: "text" },
  };
}

function defaultQuery(): QueryPanelValue {
  return { documentType: "" };
}

function normalizeQuery(value: unknown): QueryPanelValue {
  if (!value || typeof value !== "object" || Array.isArray(value)) return defaultQuery();
  const documentType = "documentType" in value && typeof value.documentType === "string" ? value.documentType : "";
  return { documentType };
}

function queryString(value: QueryPanelValue[string]) {
  return typeof value === "string" ? value : "";
}

function defaultForm(): RuleForm {
  return {
    ruleCode: "",
    documentType: "purchase_inbound",
    ruleName: "",
    template: "{OWNER}-ASN-{YYYY}{MM}{DD}-{SEQ}",
    resetPolicy: "daily",
    sequenceWidth: "4",
    enabled: true,
    effectiveFrom: "",
    effectiveTo: "",
  };
}

function formFromRule(rule: DocumentNumberRule): RuleForm {
  return {
    ruleCode: rule.rule_code,
    documentType: rule.document_type,
    ruleName: rule.rule_name,
    template: rule.template,
    resetPolicy: rule.reset_policy,
    sequenceWidth: String(rule.sequence_width),
    enabled: rule.enabled,
    effectiveFrom: toLocalDateTime(rule.effective_from),
    effectiveTo: toLocalDateTime(rule.effective_to),
  };
}

function renderPreview(template: string, sequenceWidth: number) {
  const now = new Date();
  const pad = (value: number) => String(value).padStart(2, "0");
  return template
    .replaceAll("{OWNER}", "PY001")
    .replaceAll("{YYYY}", String(now.getFullYear()))
    .replaceAll("{YY}", String(now.getFullYear()).slice(-2))
    .replaceAll("{MM}", pad(now.getMonth() + 1))
    .replaceAll("{DD}", pad(now.getDate()))
    .replaceAll("{SEQ}", String(1).padStart(sequenceWidth, "0"));
}

function documentTypeLabel(value: string, options: DocumentTypeOption[]) {
  return options.find((option) => option.value === value)?.label ?? value;
}

function formatDateTime(value: string) {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(new Date(value));
}

function toLocalDateTime(value?: string | null) {
  if (!value) return "";
  const date = new Date(value);
  const pad = (part: number) => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

function errorMessage(error: unknown, fallback: string) {
  return error instanceof Error ? error.message : fallback;
}

function NoticePanel({ notice }: { notice: Notice }) {
  if (!notice) return null;
  return <div className={cn("rounded-md border px-3 py-2 text-sm", notice.type === "success" ? "border-wms-success/30 bg-wms-success/10 text-wms-success" : "border-destructive/30 bg-destructive/10 text-destructive")} role={notice.type === "success" ? "status" : "alert"}>{notice.text}</div>;
}
