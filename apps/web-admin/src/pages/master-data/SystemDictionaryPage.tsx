import * as React from "react";
import {
  Button,
  Checkbox,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
  PageHeader,
  SystemDictionaryTwoPane,
} from "@wms/ui";
import { Ban, Pencil, Plus, RefreshCw } from "lucide-react";

import {
  useDisableSystemDictionaryItemMutation,
  useSystemDictionaryGroupsQuery,
  useUpsertSystemDictionaryItemMutation,
  type DisableSystemDictionaryItemRequest,
  type SystemDictionaryPaneGroup,
  type SystemDictionaryPaneItem,
  type UpsertSystemDictionaryItemRequest,
} from "@/features/master-data/master-data-queries";
import type { CurrentUser } from "@/features/auth/auth-queries";
import { DualPersonPolicyMatrix } from "./DualPersonPolicyMatrix";
import { PrintTemplateTypeFields } from "./PrintTemplateTypeFields";

interface SystemDictionaryMeta {
  title: string;
  emptyTitle: string;
  emptyDescription?: string;
}

interface M1SystemDictionaryPageProps {
  meta: SystemDictionaryMeta;
  currentUser: CurrentUser;
}

type ActiveDialog = "upsert" | "disable" | null;

interface ItemFormState {
  itemCode: string;
  itemName: string;
  ownerId: string;
  enabled: boolean;
  sortOrder: string;
  paramsText: string;
  effectiveFrom: string;
  effectiveTo: string;
}

interface DisableFormState {
  itemCode: string;
  itemName: string;
  ownerId: string;
  reason: string;
}

const emptySystemDictionaryGroups: SystemDictionaryPaneGroup[] = [
  { code: "document_type", name: "单据类型", items: [] },
  { code: "special_drug_category", name: "特殊药品分类", items: [] },
];
const defaultDictionaryGroupCode = "special_drug_category";

const emptyItemForm: ItemFormState = {
  itemCode: "",
  itemName: "",
  ownerId: "",
  enabled: true,
  sortOrder: "0",
  paramsText: "{}",
  effectiveFrom: "",
  effectiveTo: "",
};

const emptyDisableForm: DisableFormState = {
  itemCode: "",
  itemName: "",
  ownerId: "",
  reason: "",
};

export function M1SystemDictionaryPage({ meta, currentUser }: M1SystemDictionaryPageProps) {
  const groupsQuery = useSystemDictionaryGroupsQuery();
  const upsertMutation = useUpsertSystemDictionaryItemMutation();
  const disableMutation = useDisableSystemDictionaryItemMutation();
  const [lastEvent, setLastEvent] = React.useState<string | null>(null);
  const [activeDialog, setActiveDialog] = React.useState<ActiveDialog>(null);
  const [editingItemCode, setEditingItemCode] = React.useState<string | null>(null);
  const [itemForm, setItemForm] = React.useState<ItemFormState>(emptyItemForm);
  const [disableForm, setDisableForm] = React.useState<DisableFormState>(emptyDisableForm);
  const [dialogError, setDialogError] = React.useState<string | null>(null);
  const [selectedDictCode, setSelectedDictCode] = React.useState<string | null>(null);
  const groups = groupsQuery.data ?? emptySystemDictionaryGroups;
  const activeGroup =
    groups.find((group) => group.code === selectedDictCode) ??
    groups.find((group) => group.code === defaultDictionaryGroupCode) ??
    groups[0] ??
    emptySystemDictionaryGroups[0];
  const pending = upsertMutation.isPending || disableMutation.isPending;
  const canOwnerWrite = currentUser.permissions.includes("m1.system_dictionary.write");
  const canGlobalWrite = currentUser.permissions.includes("m1.system_dictionary.global.write");
  const canMaintain = canOwnerWrite || canGlobalWrite;

  React.useEffect(() => {
    if (groups.some((group) => group.code === selectedDictCode)) return;
    setSelectedDictCode(groups.find((group) => group.code === defaultDictionaryGroupCode)?.code ?? groups[0]?.code ?? null);
  }, [groups, selectedDictCode]);

  async function refreshRows() {
    await groupsQuery.refetch();
    setLastEvent(`${meta.title} 已刷新`);
  }

  function openCreateDialog() {
    setEditingItemCode(null);
    setItemForm({
      ...emptyItemForm,
      ownerId: canOwnerWrite && !canGlobalWrite ? currentUser.owner_id : "",
    });
    setDialogError(null);
    setActiveDialog("upsert");
  }

  function openUpsertDialog(item: SystemDictionaryPaneItem) {
    setEditingItemCode(item.code);
    setItemForm({
      itemCode: item.code,
      itemName: item.name,
      ownerId: item.ownerId ?? "",
      enabled: item.enabled,
      sortOrder: String(item.sortOrder),
      paramsText: JSON.stringify(item.params, null, 2),
      effectiveFrom: isoToDateTimeLocal(item.effectiveFrom),
      effectiveTo: isoToDateTimeLocal(item.effectiveTo),
    });
    setDialogError(null);
    setActiveDialog("upsert");
  }

  function openDisableDialog(item: SystemDictionaryPaneItem) {
    setDisableForm({
      itemCode: item.code,
      itemName: item.name,
      ownerId: item.ownerId ?? "",
      reason: item.disabledReason ?? "",
    });
    setDialogError(null);
    setActiveDialog("disable");
  }

  async function submitUpsert(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setDialogError(null);
    try {
      const itemCode = requiredText(itemForm.itemCode, "编码");
      const request: UpsertSystemDictionaryItemRequest = {
        item_name: requiredText(itemForm.itemName, "名称"),
        owner_id: nullableText(itemForm.ownerId),
        enabled: itemForm.enabled,
        sort_order: parseNonNegativeInteger(itemForm.sortOrder, "排序号"),
        params: parseJsonObject(itemForm.paramsText),
        effective_from: dateTimeLocalToIsoOrNull(itemForm.effectiveFrom),
        effective_to: dateTimeLocalToIsoOrNull(itemForm.effectiveTo),
      };
      const saved = await upsertMutation.mutateAsync({
        dictCode: activeGroup.code,
        itemCode,
        request,
      });
      await groupsQuery.refetch();
      setActiveDialog(null);
      setLastEvent(`${saved.item_code} 已保存`);
    } catch (errorValue: unknown) {
      setDialogError(errorValue instanceof Error ? errorValue.message : "保存失败");
    }
  }

  async function submitDisable(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setDialogError(null);
    try {
      const request: DisableSystemDictionaryItemRequest = {
        disabled_reason: nullableText(disableForm.reason),
        owner_id: nullableText(disableForm.ownerId),
      };
      const disabled = await disableMutation.mutateAsync({
        dictCode: activeGroup.code,
        itemCode: requiredText(disableForm.itemCode, "编码"),
        request,
      });
      await groupsQuery.refetch();
      setActiveDialog(null);
      setLastEvent(`${disabled.item_code} 已停用`);
    } catch (errorValue: unknown) {
      setDialogError(errorValue instanceof Error ? errorValue.message : "停用失败");
    }
  }

  return (
    <section className="flex w-full flex-col gap-5 px-4 py-8 lg:px-8">
      <PageHeader
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {lastEvent && (
              <span className="text-sm text-muted-foreground" role="status">
                {lastEvent}
              </span>
            )}
            <Button type="button" variant="outline" size="sm" onClick={refreshRows}>
              <RefreshCw className="size-4" aria-hidden />
              刷新
            </Button>
          </div>
        }
      />

      {groupsQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {groupsQuery.error.message}
        </div>
      )}

      <SystemDictionaryTwoPane
        groups={groups}
        selectedGroupCode={activeGroup.code}
        onSelectedGroupCodeChange={setSelectedDictCode}
        storageKey="m1-system-dictionary-two-pane"
        selectable
        loading={groupsQuery.isPending}
        error={groupsQuery.error?.message}
        onRefresh={refreshRows}
        headerActions={canMaintain ? (
          <Button type="button" size="sm" onClick={openCreateDialog}>
            <Plus className="size-4" aria-hidden />
            新增
          </Button>
        ) : null}
        renderItemActions={canMaintain ? (item) => {
          const actionItem = activeGroup.items.find(
            (candidate) => candidate.code === item.code && candidate.source === item.source,
          );
          if (!actionItem || (actionItem.ownerId ? !canOwnerWrite : !canGlobalWrite)) return null;
          return (
            <>
              <Button type="button" variant="outline" size="sm" onClick={() => openUpsertDialog(actionItem)}>
                <Pencil className="size-4" aria-hidden />
                更新
              </Button>
              <Button
                type="button"
                variant="outline"
                size="sm"
                disabled={!actionItem.enabled}
                onClick={() => openDisableDialog(actionItem)}
              >
                <Ban className="size-4" aria-hidden />
                停用
              </Button>
            </>
          );
        } : undefined}
        emptyTitle={meta.emptyTitle}
        emptyDescription={
          groupsQuery.isPending
            ? "正在读取系统字典项。"
            : (meta.emptyDescription ?? `${activeGroup.name} 暂无可展示字典项。`)
        }
      />

      {activeGroup.code === "special_drug_category" && (
        <DualPersonPolicyMatrix categories={activeGroup.items} />
      )}

      <DictionaryDialogs
        activeDialog={activeDialog}
        editing={Boolean(editingItemCode)}
        itemForm={itemForm}
        disableForm={disableForm}
        activeGroup={activeGroup}
        pending={pending}
        errorMessage={dialogError}
        onOpenChange={(open) => !open && setActiveDialog(null)}
        onItemFormChange={setItemForm}
        onDisableFormChange={setDisableForm}
        onSubmitUpsert={submitUpsert}
        onSubmitDisable={submitDisable}
      />
    </section>
  );
}

function DictionaryDialogs({
  activeDialog,
  editing,
  itemForm,
  disableForm,
  activeGroup,
  pending,
  errorMessage,
  onOpenChange,
  onItemFormChange,
  onDisableFormChange,
  onSubmitUpsert,
  onSubmitDisable,
}: {
  activeDialog: ActiveDialog;
  editing: boolean;
  itemForm: ItemFormState;
  disableForm: DisableFormState;
  activeGroup: SystemDictionaryPaneGroup;
  pending: boolean;
  errorMessage: string | null;
  onOpenChange: (open: boolean) => void;
  onItemFormChange: React.Dispatch<React.SetStateAction<ItemFormState>>;
  onDisableFormChange: React.Dispatch<React.SetStateAction<DisableFormState>>;
  onSubmitUpsert: (event: React.FormEvent<HTMLFormElement>) => void;
  onSubmitDisable: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  if (!activeDialog) return null;
  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        {errorMessage && (
          <div
            className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            role="alert"
          >
            {errorMessage}
          </div>
        )}

        {activeDialog === "upsert" && (
          <form className="grid gap-3 md:grid-cols-2" onSubmit={onSubmitUpsert}>
            <DialogHeader className="md:col-span-2">
              <DialogTitle>{editing ? "更新字典项" : "新增字典项"}</DialogTitle>
              <DialogDescription>
                {activeGroup.name} / {activeGroup.code}
              </DialogDescription>
            </DialogHeader>
            <TextField
              label="编码"
              value={itemForm.itemCode}
              readOnly={editing}
              onChange={(itemCode) => onItemFormChange((value) => ({ ...value, itemCode }))}
            />
            <TextField
              label="名称"
              value={itemForm.itemName}
              onChange={(itemName) => onItemFormChange((value) => ({ ...value, itemName }))}
            />
            <TextField
              label="货主 ID（可选）"
              value={itemForm.ownerId}
              onChange={(ownerId) => onItemFormChange((value) => ({ ...value, ownerId }))}
            />
            <TextField
              label="排序号"
              type="number"
              value={itemForm.sortOrder}
              onChange={(sortOrder) => onItemFormChange((value) => ({ ...value, sortOrder }))}
            />
            <div className="flex items-center gap-2 self-end rounded-md border px-3 py-2">
              <Checkbox
                id="dictionary-item-enabled"
                checked={itemForm.enabled}
                onCheckedChange={(checked) =>
                  onItemFormChange((value) => ({ ...value, enabled: checked === true }))
                }
              />
              <Label htmlFor="dictionary-item-enabled">启用</Label>
            </div>
            <TextField
              label="生效开始"
              type="datetime-local"
              value={itemForm.effectiveFrom}
              onChange={(effectiveFrom) =>
                onItemFormChange((value) => ({ ...value, effectiveFrom }))
              }
            />
            <TextField
              label="生效结束"
              type="datetime-local"
              value={itemForm.effectiveTo}
              onChange={(effectiveTo) => onItemFormChange((value) => ({ ...value, effectiveTo }))}
            />
            {activeGroup.code === "print_template_type" ? (
              <PrintTemplateTypeFields
                value={itemForm.paramsText}
                onChange={(paramsText) =>
                  onItemFormChange((current) => ({ ...current, paramsText }))
                }
              />
            ) : (
              <label className="grid gap-1 text-xs text-muted-foreground md:col-span-2">
                <span>
                  参数 JSON
                  <span className="ml-1 rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
                    高级
                  </span>
                </span>
                <textarea
                  className={[
                    "min-h-40 rounded-md border border-input bg-background px-3 py-2",
                    "font-mono text-sm text-foreground shadow-sm",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  ].join(" ")}
                  value={itemForm.paramsText}
                  onChange={(event) =>
                    onItemFormChange((value) => ({ ...value, paramsText: event.target.value }))
                  }
                />
              </label>
            )}
            <DialogFooter className="md:col-span-2">
              <CancelButton />
              <Button type="submit" disabled={pending}>
                <Pencil className="size-4" aria-hidden />
                保存
              </Button>
            </DialogFooter>
          </form>
        )}

        {activeDialog === "disable" && (
          <form className="grid gap-3" onSubmit={onSubmitDisable}>
            <DialogHeader>
              <DialogTitle>停用字典项</DialogTitle>
              <DialogDescription>
                {activeGroup.name} / {disableForm.itemCode} / {disableForm.itemName}
              </DialogDescription>
            </DialogHeader>
            <TextField
              label="货主 ID（可选）"
              value={disableForm.ownerId}
              onChange={(ownerId) => onDisableFormChange((value) => ({ ...value, ownerId }))}
            />
            <TextField
              label="停用原因"
              value={disableForm.reason}
              onChange={(reason) => onDisableFormChange((value) => ({ ...value, reason }))}
            />
            <DialogFooter>
              <CancelButton />
              <Button type="submit" variant="destructive" disabled={pending}>
                <Ban className="size-4" aria-hidden />
                确认停用
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}

function TextField({
  label,
  value,
  onChange,
  type = "text",
  readOnly = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: React.HTMLInputTypeAttribute;
  readOnly?: boolean;
}) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Input
        type={type}
        value={value}
        readOnly={readOnly}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

function CancelButton() {
  return (
    <DialogClose asChild>
      <Button type="button" variant="outline">
        取消
      </Button>
    </DialogClose>
  );
}

function requiredText(value: string, field: string) {
  const trimmed = value.trim();
  if (!trimmed) throw new Error(`${field} 必填`);
  return trimmed;
}

function nullableText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function parseNonNegativeInteger(value: string, field: string) {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 0) throw new Error(`${field}必须是非负整数`);
  return parsed;
}

function parseJsonObject(value: string): Record<string, unknown> {
  const trimmed = value.trim();
  if (!trimmed) return {};
  const parsed: unknown = JSON.parse(trimmed);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("参数必须是 JSON 对象");
  }
  return parsed as Record<string, unknown>;
}

function dateTimeLocalToIsoOrNull(value: string) {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) throw new Error("时间格式无效");
  return date.toISOString();
}

function isoToDateTimeLocal(value?: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}
