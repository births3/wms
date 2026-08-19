import * as React from "react";
import {
  Button,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  StatusBadge,
  type DataGridColumn,
  type DataGridCreateAction,
  type DataGridRefreshAction,
  type DataGridToolbarAction,
} from "@wms/ui";
import { FlaskConical, Power, Upload } from "lucide-react";

import { useCustomerAddressesQuery } from "@/features/master-data/master-data-queries/queries";
import {
  useCreatePrintSuiteDraftMutation,
  useDisablePrintSuiteMutation,
  usePrintDocumentCategoriesQuery,
  usePrintSuiteInstancesQuery,
  usePrintSuitesQuery,
  usePublishPrintSuiteMutation,
  useTestPrintSuiteMutation,
  type CreatePrintSuiteDraftRequest,
  type DeliveryNoteGroupListItem,
  type PrintDocumentCategory,
  type PrintSuiteInstance,
  type PrintSuiteItemInput,
  type PrintSuiteTestResult,
  type PrintSuiteVersion,
} from "@/features/print-orchestration/print-orchestration-queries";
import { usePrintTemplatesQuery } from "@/features/print-template/print-template-queries";
import { formatDateTime } from "@/lib/format";
import { BUTTON_REFRESH, COLUMN_CREATED_AT, COLUMN_STATUS, COLUMN_VERSION, COLUMN_WAREHOUSE, FIELD_VALIDITY, LOADING_SAVING } from "@/lib/ui-strings";
import {
  H9LifecycleConfirmDialog,
  type H9LifecycleConfirmation,
} from "./H9LifecycleConfirmDialog";
import { H9CategoryPdfPanel } from "./H9CategoryPdfPanel";
import { instanceStatusLabel, statusCompletion, statusLabel } from "./print-status";

/**
 * US-H9-008 打印组套页签：版本列表 + 冻结组套实例；新建/测试走 Dialog。
 * 分类来自 M1 系统字典 print_document_category；rendered 分类绑定已发布模板
 * 版本，external_file 分类绑定稳定的 H-FILE 文件引用（禁止临时 URL）。
 */

export interface H9SuiteSelectOption {
  label: string;
  value: string;
}

interface H9PrintSuitePanelProps {
  canWrite: boolean;
  canReadPdf: boolean;
  canPreparePdf: boolean;
  canDownloadPdf: boolean;
  canEmergencyPrintPdf: boolean;
  warehouses: H9SuiteSelectOption[];
  customers: H9SuiteSelectOption[];
  groups: DeliveryNoteGroupListItem[];
  onNotice: (message: string) => void;
}

interface SuiteLifecycleAction {
  kind: "publish" | "disable";
  suite: PrintSuiteVersion;
}

export function H9PrintSuitePanel({
  canWrite,
  canReadPdf,
  canPreparePdf,
  canDownloadPdf,
  canEmergencyPrintPdf,
  warehouses,
  customers,
  groups,
  onNotice,
}: H9PrintSuitePanelProps) {
  const suitesQuery = usePrintSuitesQuery();
  const categoriesQuery = usePrintDocumentCategoriesQuery();
  const instancesQuery = usePrintSuiteInstancesQuery(null);
  const createSuite = useCreatePrintSuiteDraftMutation();
  const testSuite = useTestPrintSuiteMutation();
  const publishSuite = usePublishPrintSuiteMutation();
  const disableSuite = useDisablePrintSuiteMutation();
  const [suiteIds, setSuiteIds] = React.useState<string[]>([]);
  const [createOpen, setCreateOpen] = React.useState(false);
  const [testOpen, setTestOpen] = React.useState(false);
  const [lifecycleAction, setLifecycleAction] = React.useState<SuiteLifecycleAction | null>(null);
  const suites = suitesQuery.data ?? [];
  const selectedSuite = suites.find((item) => suiteIds.includes(item.id)) ?? null;

  const createAction: DataGridCreateAction = {
    label: "新建组套版本",
    description: "从 M1 受控单据分类创建下一版打印组套草稿",
    disabled: !canWrite || createSuite.isPending,
    onClick: () => {
      createSuite.reset();
      setCreateOpen(true);
    },
  };
  const testAction: DataGridToolbarAction = {
    key: "test-suite",
    label: "测试组套",
    description: "对真实归集组做就绪性/完整性预检并展示样本解析",
    icon: <FlaskConical className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedSuite || !["draft", "tested"].includes(selectedSuite.status) || testSuite.isPending,
    onClick: () => {
      testSuite.reset();
      setTestOpen(true);
    },
  };
  const publishAction: DataGridToolbarAction = {
    key: "publish-suite",
    label: "发布组套",
    description: "发布已测试版本；同级同对象有效期重叠会被拒绝",
    icon: <Upload className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedSuite || selectedSuite.status !== "tested" || publishSuite.isPending,
    onClick: () => {
      if (!selectedSuite) return;
      publishSuite.reset();
      setLifecycleAction({ kind: "publish", suite: selectedSuite });
    },
  };
  const disableAction: DataGridToolbarAction = {
    key: "disable-suite",
    label: "停用组套",
    description: "停用当前发布版本；既有组套实例快照不受影响",
    icon: <Power className="size-4" aria-hidden />,
    disabled: !canWrite || !selectedSuite || selectedSuite.status !== "published" || disableSuite.isPending,
    onClick: () => {
      if (!selectedSuite) return;
      disableSuite.reset();
      setLifecycleAction({ kind: "disable", suite: selectedSuite });
    },
  };

  const lifecycleConfirmation: H9LifecycleConfirmation | null = lifecycleAction
    ? {
        title: lifecycleAction.kind === "publish" ? "发布打印组套" : "停用打印组套",
        description: lifecycleAction.kind === "publish"
          ? `确认发布打印组套 V${lifecycleAction.suite.version_no}「${lifecycleAction.suite.name}」？发布后将参与新组套实例解析。`
          : `确认停用打印组套 V${lifecycleAction.suite.version_no}「${lifecycleAction.suite.name}」？既有实例快照不受影响。`,
        confirmLabel: lifecycleAction.kind === "publish" ? "确认发布" : "确认停用",
        destructive: lifecycleAction.kind === "disable",
      }
    : null;

  async function confirmLifecycleAction() {
    if (!lifecycleAction) return;
    const suite = lifecycleAction.kind === "publish"
      ? await publishSuite.mutateAsync(lifecycleAction.suite.id)
      : await disableSuite.mutateAsync(lifecycleAction.suite.id);
    setSuiteIds([]);
    setLifecycleAction(null);
    onNotice(`打印组套 V${suite.version_no} 已${lifecycleAction.kind === "publish" ? "发布" : "停用"}`);
  }

  return (
    <div className="space-y-5">
      <p className="text-sm text-muted-foreground">
        组套匹配顺序固定为送货地址、客户、线路、货主 + 仓库默认；已发布版本不可改写，
        策略在截单生成组套实例时冻结。
      </p>
      {(suitesQuery.error || categoriesQuery.error || publishSuite.error || disableSuite.error) && (
        <p className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive" role="alert">
          {suitesQuery.error?.message ?? categoriesQuery.error?.message ?? publishSuite.error?.message ?? disableSuite.error?.message}
        </p>
      )}
      <DataGrid
        columns={suiteColumns(customers, warehouses)}
        data={suites}
        rowKey={(row) => row.id}
        storageKey="h9-print-suites"
        emptyTitle="暂无打印组套版本"
        emptyDescription="未发布组套时，截单只生成随货同行单归集组，不创建组套实例"
        caption={suitesQuery.isPending ? "加载打印组套..." : undefined}
        refreshAction={refreshAction(suitesQuery, "打印组套")}
        createAction={createAction}
        toolbarActions={[testAction, publishAction, disableAction]}
        selectedRowKeys={suiteIds}
        onSelectedRowKeysChange={(keys) => setSuiteIds(keys.length ? [keys.at(-1) as string] : [])}
        selectable
        tableClassName="min-w-[1400px]"
      />
      <section className="space-y-2">
        <h3 className="text-sm font-medium">组套实例（截单后按解析结果冻结）</h3>
        <DataGrid
          columns={instanceColumns}
          data={instancesQuery.data ?? []}
          rowKey={(row) => row.id}
          storageKey="h9-print-suite-instances"
          emptyTitle="暂无组套实例"
          emptyDescription="发布组套后，新的截单会按送货地址、客户、线路、仓库默认解析并冻结实例"
          refreshAction={refreshAction(instancesQuery, "组套实例")}
          tableClassName="min-w-[1200px]"
        />
      </section>
      <H9CategoryPdfPanel
        instances={instancesQuery.data ?? []}
        categories={categoriesQuery.data ?? []}
        canRead={canReadPdf}
        canPrepare={canPreparePdf}
        canDownload={canDownloadPdf}
        canEmergencyPrint={canEmergencyPrintPdf}
        onNotice={onNotice}
      />
      <PrintSuiteCreateDialog
        open={createOpen}
        pending={createSuite.isPending}
        errorMessage={createSuite.error?.message ?? categoriesQuery.error?.message}
        warehouses={warehouses}
        customers={customers}
        categories={categoriesQuery.data ?? []}
        onOpenChange={setCreateOpen}
        onSubmit={async (request) => {
          const suite = await createSuite.mutateAsync(request);
          setCreateOpen(false);
          onNotice(`打印组套草稿 V${suite.version_no}「${suite.name}」已保存`);
        }}
      />
      <PrintSuiteTestDialog
        open={testOpen}
        pending={testSuite.isPending}
        errorMessage={testSuite.error?.message}
        suite={selectedSuite}
        groups={groups}
        result={testSuite.data ?? null}
        onOpenChange={(next) => {
          setTestOpen(next);
          if (!next) testSuite.reset();
        }}
        onSubmit={async (groupIds) => {
          if (!selectedSuite) return;
          const result = await testSuite.mutateAsync({ versionId: selectedSuite.id, groupIds });
          onNotice(`组套 V${result.suite.version_no} 测试完成：${result.samples.length} 个样本组`);
        }}
      />
      <H9LifecycleConfirmDialog
        confirmation={lifecycleConfirmation}
        pending={publishSuite.isPending || disableSuite.isPending}
        errorMessage={lifecycleAction?.kind === "publish" ? publishSuite.error?.message : disableSuite.error?.message}
        onOpenChange={(open) => {
          if (!open) setLifecycleAction(null);
        }}
        onConfirm={() => void confirmLifecycleAction().catch(() => undefined)}
      />
    </div>
  );
}

interface PrintSuiteCreateDialogProps {
  open: boolean;
  pending: boolean;
  errorMessage?: string;
  warehouses: H9SuiteSelectOption[];
  customers: H9SuiteSelectOption[];
  categories: PrintDocumentCategory[];
  onOpenChange: (open: boolean) => void;
  onSubmit: (request: CreatePrintSuiteDraftRequest) => Promise<void>;
}

interface SuiteItemDraft {
  categoryCode: string;
  copies: number;
  outputSlot: string;
  required: boolean;
  readyPolicy: PrintSuiteItemInput["ready_policy"];
  failurePolicy: PrintSuiteItemInput["failure_policy"];
  templateVersionId: string;
  externalFileRef: string;
}

export function PrintSuiteCreateDialog({
  open,
  pending,
  errorMessage,
  warehouses,
  customers,
  categories,
  onOpenChange,
  onSubmit,
}: PrintSuiteCreateDialogProps) {
  const [name, setName] = React.useState("");
  const [warehouseId, setWarehouseId] = React.useState("");
  const [scope, setScope] = React.useState<CreatePrintSuiteDraftRequest["scope"]>("customer");
  const [customerId, setCustomerId] = React.useState("");
  const [addressId, setAddressId] = React.useState("");
  const [routeCode, setRouteCode] = React.useState("");
  const [effectiveFrom, setEffectiveFrom] = React.useState("");
  const [effectiveTo, setEffectiveTo] = React.useState("");
  const [items, setItems] = React.useState<SuiteItemDraft[]>([]);
  const templatesQuery = usePrintTemplatesQuery();
  const publishedTemplates = React.useMemo(
    () => (templatesQuery.data ?? []).filter(
      (row) => row.enabled && row.latestVersionStatus === "published",
    ),
    [templatesQuery.data],
  );
  const needsCustomer = scope === "delivery_address" || scope === "customer";
  const addressesQuery = useCustomerAddressesQuery(
    scope === "delivery_address" && customerId ? customerId : null,
  );
  const addresses = addressesQuery.data ?? [];

  React.useEffect(() => {
    if (!open) return;
    setName("");
    setWarehouseId(warehouses[0]?.value ?? "");
    setScope("customer");
    setCustomerId(customers[0]?.value ?? "");
    setAddressId("");
    setRouteCode("");
    setEffectiveFrom(localDateTime(new Date()));
    setEffectiveTo("");
    setItems([]);
  }, [customers, open, warehouses]);

  React.useEffect(() => {
    if (addresses.length > 0 && !addresses.some((item) => item.id === addressId)) {
      setAddressId(addresses[0].id);
    }
  }, [addressId, addresses]);

  const categoryOf = (code: string) => categories.find((item) => item.item_code === code);

  function addItem(code: string) {
    const category = categoryOf(code);
    if (!category) return;
    setItems((current) => [
      ...current,
      {
        categoryCode: code,
        copies: 1,
        outputSlot: "tray-1",
        required: true,
        readyPolicy: "wait_hold_instance",
        failurePolicy: "pause_suite",
        templateVersionId: category.source_mode === "rendered"
          ? publishedTemplates.find((template) => template.templateTypeCode === code)?.latestVersionId ?? ""
          : "",
        externalFileRef: category.source_mode === "external_file" ? `h-file:${code}` : "",
      },
    ]);
  }

  function updateItem(index: number, patch: Partial<SuiteItemDraft>) {
    setItems((current) =>
      current.map((item, itemIndex) => (itemIndex === index ? { ...item, ...patch } : item)),
    );
  }

  const itemsValid = items.length > 0 && items.every((item) => {
    const category = categoryOf(item.categoryCode);
    if (!category || item.copies < 1 || !item.outputSlot.trim()) return false;
    if (item.required && item.failurePolicy !== "pause_suite") return false;
    return category.source_mode === "rendered"
      ? Boolean(item.templateVersionId)
      : item.externalFileRef.trim().startsWith("h-file:");
  });

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!name.trim() || !warehouseId || !effectiveFrom || !itemsValid) return;
    if (needsCustomer && !customerId) return;
    if (scope === "delivery_address" && !addressId) return;
    if (scope === "route" && !routeCode.trim()) return;
    await onSubmit({
      name: name.trim(),
      warehouse_id: warehouseId,
      scope,
      customer_id: needsCustomer ? customerId : null,
      delivery_address_id: scope === "delivery_address" ? addressId : null,
      route_code: scope === "route" ? routeCode.trim() : null,
      effective_from: new Date(effectiveFrom).toISOString(),
      effective_to: effectiveTo ? new Date(effectiveTo).toISOString() : null,
      items: items.map((item, index) => {
        const category = categoryOf(item.categoryCode);
        const rendered = category?.source_mode === "rendered";
        return {
          category_code: item.categoryCode,
          copies: item.copies,
          sort_order: index + 1,
          output_slot: item.outputSlot.trim(),
          required: item.required,
          ready_policy: item.readyPolicy,
          failure_policy: item.failurePolicy,
          source_mode: rendered ? "rendered" : "external_file",
          template_version_id: rendered ? item.templateVersionId : null,
          external_file_ref: rendered ? null : item.externalFileRef.trim(),
        } satisfies PrintSuiteItemInput;
      }),
    });
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>新建打印组套版本</DialogTitle>
          <DialogDescription>
            单据分类来自 M1 系统字典；rendered 分类必须绑定已发布模板版本，
            external_file 分类绑定稳定 H-FILE 文件引用，不接受临时外部 URL。
            必需打印项不可配置为跳过。
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-5" onSubmit={(event) => void submit(event).catch(() => undefined)}>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="组套名称"><Input value={name} maxLength={100} onChange={(event) => setName(event.target.value)} /></Field>
            <Field label={COLUMN_WAREHOUSE}><NativeSelect value={warehouseId} options={warehouses} onChange={setWarehouseId} /></Field>
            <Field label="适用层级">
              <NativeSelect
                value={scope}
                options={[
                  { value: "delivery_address", label: "送货地址" },
                  { value: "customer", label: "客户" },
                  { value: "route", label: "线路" },
                  { value: "warehouse_default", label: "货主加仓库默认" },
                ]}
                onChange={(value) => setScope(value as CreatePrintSuiteDraftRequest["scope"])}
              />
            </Field>
            {needsCustomer && (
              <Field label="客户"><NativeSelect value={customerId} options={customers} onChange={setCustomerId} /></Field>
            )}
            {scope === "delivery_address" && (
              <Field label="送货地址" wide>
                <NativeSelect
                  value={addressId}
                  options={addresses.map((item) => ({
                    value: item.id,
                    label: `${item.province}${item.city}${item.district}${item.detail_address}`,
                  }))}
                  onChange={setAddressId}
                />
              </Field>
            )}
            {scope === "route" && (
              <Field label="线路编码"><Input value={routeCode} maxLength={64} onChange={(event) => setRouteCode(event.target.value)} /></Field>
            )}
            <Field label="生效时间"><Input type="datetime-local" value={effectiveFrom} onChange={(event) => setEffectiveFrom(event.target.value)} /></Field>
            <Field label="失效时间（可选）"><Input type="datetime-local" value={effectiveTo} onChange={(event) => setEffectiveTo(event.target.value)} /></Field>
          </div>
          <fieldset className="space-y-3 rounded-md border p-4">
            <legend className="px-1 text-sm font-medium">有序打印项</legend>
            <Field label="添加打印项分类">
              <NativeSelect
                value=""
                options={categories.map((category) => ({
                  value: category.item_code,
                  label: `${category.item_name}（${sourceModeLabel(category.source_mode)}）`,
                }))}
                onChange={(code) => code && addItem(code)}
              />
            </Field>
            {items.length === 0 && (
              <p className="text-sm text-muted-foreground">尚未添加打印项；同一分类允许多项。</p>
            )}
            <ol className="space-y-3" aria-label="已选打印项顺序">
              {items.map((item, index) => {
                const category = categoryOf(item.categoryCode);
                const rendered = category?.source_mode === "rendered";
                return (
                  <li key={`${item.categoryCode}-${index}`} className="space-y-3 rounded-md border bg-muted/40 p-3">
                    <div className="flex items-center gap-2 text-sm">
                      <span className="font-medium">
                        {index + 1}. {category?.item_name ?? item.categoryCode} · {sourceModeLabel(category?.source_mode ?? "rendered")}
                      </span>
                      <span className="flex-1" />
                      <Button type="button" variant="ghost" size="sm" disabled={index === 0} onClick={() => setItems((current) => move(current, index, -1))}>上移</Button>
                      <Button type="button" variant="ghost" size="sm" disabled={index === items.length - 1} onClick={() => setItems((current) => move(current, index, 1))}>下移</Button>
                      <Button type="button" variant="ghost" size="sm" onClick={() => setItems((current) => current.filter((_, itemIndex) => itemIndex !== index))}>移除</Button>
                    </div>
                    <div className="grid gap-3 sm:grid-cols-3">
                      <Field label={`份数（第 ${index + 1} 项）`}>
                        <Input
                          type="number"
                          min={1}
                          max={20}
                          value={item.copies}
                          onChange={(event) => updateItem(index, { copies: Number(event.target.value) || 1 })}
                        />
                      </Field>
                      <Field label={`逻辑输出槽（第 ${index + 1} 项）`}>
                        <Input value={item.outputSlot} maxLength={64} onChange={(event) => updateItem(index, { outputSlot: event.target.value })} />
                      </Field>
                      <label className="flex items-center gap-2 pt-6 text-sm">
                        <input
                          type="checkbox"
                          checked={item.required}
                          onChange={(event) => updateItem(index, {
                            required: event.target.checked,
                            failurePolicy: event.target.checked ? "pause_suite" : item.failurePolicy,
                          })}
                        />
                        必需项
                      </label>
                      <Field label={`就绪策略（第 ${index + 1} 项）`}>
                        <NativeSelect
                          value={item.readyPolicy}
                          options={[
                            { value: "wait_hold_instance", label: "仅挂起当前实例" },
                            { value: "pause_agent_queue", label: "暂停对应 Agent 队列" },
                          ]}
                          onChange={(value) => updateItem(index, { readyPolicy: value as SuiteItemDraft["readyPolicy"] })}
                        />
                      </Field>
                      <Field label={`失败策略（第 ${index + 1} 项）`}>
                        <NativeSelect
                          value={item.failurePolicy}
                          options={[
                            { value: "pause_suite", label: "暂停组套" },
                            ...(item.required ? [] : [{ value: "skip_and_continue", label: "跳过并继续（仅非必需）" }]),
                          ]}
                          onChange={(value) => updateItem(index, { failurePolicy: value as SuiteItemDraft["failurePolicy"] })}
                        />
                      </Field>
                      {rendered ? (
                        <Field label={`模板版本（第 ${index + 1} 项）`}>
                          <NativeSelect
                            value={item.templateVersionId}
                            options={publishedTemplates
                              .filter((template) => template.templateTypeCode === item.categoryCode)
                              .map((template) => ({
                                value: template.latestVersionId,
                                label: `${template.templateName} V${template.latestVersionNo}`,
                              }))}
                            onChange={(value) => updateItem(index, { templateVersionId: value })}
                          />
                        </Field>
                      ) : (
                        <Field label={`H-FILE 文件引用（第 ${index + 1} 项）`}>
                          <Input value={item.externalFileRef} maxLength={200} onChange={(event) => updateItem(index, { externalFileRef: event.target.value })} />
                        </Field>
                      )}
                    </div>
                  </li>
                );
              })}
            </ol>
          </fieldset>
          {errorMessage && <p className="text-sm text-destructive" role="alert">{errorMessage}</p>}
          <DialogFooter>
            <DialogClose asChild><Button type="button" variant="outline" disabled={pending}>取消</Button></DialogClose>
            <Button type="submit" disabled={pending || !name.trim() || !warehouseId || !itemsValid}>
              {pending ? LOADING_SAVING : "保存草稿"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PrintSuiteTestDialogProps {
  open: boolean;
  pending: boolean;
  errorMessage?: string;
  suite: PrintSuiteVersion | null;
  groups: DeliveryNoteGroupListItem[];
  result: PrintSuiteTestResult | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (groupIds: string[]) => Promise<void>;
}

export function PrintSuiteTestDialog({
  open,
  pending,
  errorMessage,
  suite,
  groups,
  result,
  onOpenChange,
  onSubmit,
}: PrintSuiteTestDialogProps) {
  const [groupIds, setGroupIds] = React.useState<string[]>([]);

  React.useEffect(() => {
    if (open) setGroupIds([]);
  }, [open]);

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>样本归集组测试</DialogTitle>
          <DialogDescription>
            组套 {suite ? `V${suite.version_no}「${suite.name}」` : "-"} ·
            对真实随货同行单归集组执行就绪性/完整性预检，并展示按送货地址、客户、线路、
            仓库默认顺序的解析层级。
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <p className="text-sm font-medium">选择样本归集组</p>
          <ul className="max-h-48 space-y-1 overflow-y-auto rounded-md border p-2" aria-label="样本归集组">
            {groups.map((group) => (
              <li key={group.id}>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={groupIds.includes(group.id)}
                    onChange={(event) =>
                      setGroupIds((current) =>
                        event.target.checked
                          ? [...current, group.id]
                          : current.filter((id) => id !== group.id),
                      )
                    }
                  />
                  <span className="font-mono">{group.delivery_note_no}</span>
                  <span className="text-muted-foreground">{group.customer_name} · {group.order_nos.join("、")}</span>
                </label>
              </li>
            ))}
            {groups.length === 0 && <li className="p-2 text-sm text-muted-foreground">当前仓库暂无归集组可作样本</li>}
          </ul>
          {result && (
            <div className="space-y-2" aria-label="就绪预检结果">
              <p className="text-sm font-medium">预检结果（{result.samples.length} 个样本组）</p>
              {result.samples.map((sample) => (
                <div key={sample.group_id} className="space-y-2 rounded-md border bg-muted/40 p-3 text-sm">
                  <p className="font-medium">
                    {sample.delivery_note_no} · 解析层级：{sample.resolved_scope ? scopeLabel(sample.resolved_scope) : "无匹配组套"}
                    {sample.matches_this_version ? "（命中本版本）" : "（未命中本版本）"}
                  </p>
                  <ul className="space-y-1">
                    {sample.item_readiness.map((item, index) => (
                      <li key={index} className="text-muted-foreground">
                        {index + 1}. {item.category_name}（{sourceModeLabel(item.source_mode)}
                        {item.required ? "，必需" : "，可选"}）：
                        {item.ready
                          ? `就绪${item.file_bindings.length ? `，绑定 ${item.file_bindings.length} 个权威文件` : ""}`
                          : `未就绪：${item.missing.join("；")}`}
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          )}
          {errorMessage && <p className="text-sm text-destructive" role="alert">{errorMessage}</p>}
        </div>
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline" disabled={pending}>关闭</Button>
          </DialogClose>
          <Button
            type="button"
            disabled={pending || !suite || groupIds.length === 0}
            onClick={() => void onSubmit(groupIds).catch(() => undefined)}
          >
            {pending ? "测试中..." : "执行测试"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function suiteColumns(
  customers: H9SuiteSelectOption[],
  warehouses: H9SuiteSelectOption[],
): DataGridColumn<PrintSuiteVersion>[] {
  const customerLabels = new Map(customers.map((item) => [item.value, item.label]));
  const warehouseLabels = new Map(warehouses.map((item) => [item.value, item.label]));
  return [
    { key: "version_no", header: COLUMN_VERSION, width: 80, mono: true, render: (row) => `V${row.version_no}` },
    { key: "name", header: "组套名称", width: 200, render: (row) => row.name },
    { key: "status", header: COLUMN_STATUS, width: 110, render: (row) => <StatusBadge status={statusCompletion(row.status)} label={statusLabel(row.status)} size="sm" /> },
    { key: "scope", header: "匹配层级", width: 130, render: (row) => scopeLabel(row.scope) },
    {
      key: "target",
      header: "匹配对象",
      width: 220,
      render: (row) =>
        row.scope === "route"
          ? row.route_code ?? "-"
          : row.customer_id
            ? customerLabels.get(row.customer_id) ?? row.customer_id
            : "当前货主 + 仓库",
    },
    {
      key: "items",
      header: "打印项（顺序 · 份数 · 就绪策略）",
      width: 420,
      render: (row) =>
        row.items
          .map((item) => `${item.sort_order}.${item.category_name}×${item.copies}${item.required ? "（必需，" : "（可选，"}${readyPolicyLabel(item.ready_policy)}）`)
          .join(" → "),
    },
    { key: "warehouse", header: COLUMN_WAREHOUSE, width: 180, defaultHidden: true, render: (row) => warehouseLabels.get(row.warehouse_id) ?? row.warehouse_id },
    { key: "effective", header: FIELD_VALIDITY, width: 320, defaultHidden: true, render: (row) => `${formatDateTime(row.effective_from)} 至 ${formatDateTime(row.effective_to)}` },
    { key: "published_at", header: "发布时间", width: 170, render: (row) => row.published_at ? formatDateTime(row.published_at) : "-" },
    { key: "created_at", header: COLUMN_CREATED_AT, width: 170, defaultHidden: true, render: (row) => formatDateTime(row.created_at) },
  ];
}

const instanceColumns: DataGridColumn<PrintSuiteInstance>[] = [
  { key: "delivery_note_no", header: "随货同行单号", width: 250, mono: true, copyValue: (row) => row.delivery_note_no },
  { key: "suite_version_no", header: "组套版本", width: 110, mono: true, render: (row) => `V${row.suite_version_no}` },
  { key: "status", header: "实例状态", width: 130, render: (row) => <StatusBadge status={row.status === "queued" ? "completed" : "pending"} label={instanceStatusLabel(row.status)} size="sm" /> },
  { key: "hold_scope", header: "未就绪策略", width: 170, render: (row) => row.hold_scope === "agent_queue" ? "暂停 Agent 队列" : row.hold_scope === "instance" ? "挂起当前实例" : "-" },
  {
    key: "items",
    header: "打印项就绪",
    width: 360,
    render: (row) => row.items.map((item) => `${item.sort_order}.${item.category_code}${item.ready ? "✓" : "✗"}`).join(" → "),
  },
  { key: "rule", header: "归集规则版本", width: 130, defaultHidden: true, render: (row) => row.aggregation_rule_version_no ? `V${row.aggregation_rule_version_no}` : "-" },
  { key: "created_at", header: "冻结时间", width: 170, render: (row) => formatDateTime(row.created_at) },
];

export function scopeLabel(scope: PrintSuiteVersion["scope"]) {
  return scope === "delivery_address"
    ? "送货地址"
    : scope === "customer"
      ? "客户"
      : scope === "route"
        ? "线路"
        : "仓库默认";
}

function sourceModeLabel(mode: PrintDocumentCategory["source_mode"]) {
  return mode === "rendered" ? "渲染" : "外部文件";
}

function readyPolicyLabel(policy: PrintSuiteItemInput["ready_policy"]) {
  return policy === "pause_agent_queue" ? "暂停队列" : "挂起实例";
}

function refreshAction(query: { isFetching: boolean; refetch: () => Promise<unknown> }, label: string): DataGridRefreshAction {
  return { label: BUTTON_REFRESH, description: `刷新${label}`, disabled: query.isFetching, onClick: () => void query.refetch() };
}

function move<T>(list: T[], index: number, delta: number) {
  const next = [...list];
  const target = index + delta;
  if (target < 0 || target >= next.length) return next;
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

function Field({ label, wide, children }: { label: string; wide?: boolean; children: React.ReactNode }) {
  return <label className={`space-y-2 text-sm font-medium${wide ? " sm:col-span-2" : ""}`}><span>{label}</span>{children}</label>;
}

function NativeSelect({ value, options, onChange }: { value: string; options: H9SuiteSelectOption[]; onChange: (value: string) => void }) {
  return (
    <select
      className="h-10 w-full rounded-md border border-input bg-background px-3 text-sm"
      value={value}
      onChange={(event) => onChange(event.target.value)}
    >
      <option value="">请选择</option>
      {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
    </select>
  );
}

function localDateTime(date: Date) {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}
