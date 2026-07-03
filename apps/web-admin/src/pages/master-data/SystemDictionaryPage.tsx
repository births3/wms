import * as React from "react";
import {
  Button,
  Card,
  CardContent,
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
import { ArrowLeft, Ban, Pencil, Plus, RefreshCw } from "lucide-react";

import {
  useDisableSystemDictionaryItemMutation,
  useSystemDictionaryGroupsQuery,
  useUpsertSystemDictionaryItemMutation,
  type DisableSystemDictionaryItemRequest,
  type SystemDictionaryPaneGroup,
  type SystemDictionaryPaneItem,
  type UpsertSystemDictionaryItemRequest,
} from "@/features/master-data/master-data-queries";

interface SystemDictionaryMeta {
  title: string;
  subtitle: string;
  emptyTitle: string;
}

interface M1SystemDictionaryPageProps {
  meta: SystemDictionaryMeta;
  onBack: () => void;
}

type ActiveDialog = "upsert" | "disable" | null;

interface ItemFormState {
  itemCode: string;
  itemName: string;
  ownerId: string;
  enabled: boolean;
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

const emptyItemForm: ItemFormState = {
  itemCode: "",
  itemName: "",
  ownerId: "",
  enabled: true,
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

export function M1SystemDictionaryPage({ meta, onBack }: M1SystemDictionaryPageProps) {
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
    groups.find((group) => group.code === selectedDictCode) ?? groups[0] ?? emptySystemDictionaryGroups[0];
  const totalCount = groups.reduce((count, group) => count + group.items.length, 0);
  const activeCount = groups.reduce(
    (count, group) => count + group.items.filter((item) => item.enabled).length,
    0,
  );
  const pending = upsertMutation.isPending || disableMutation.isPending;

  React.useEffect(() => {
    if (groups.some((group) => group.code === selectedDictCode)) return;
    setSelectedDictCode(groups[0]?.code ?? null);
  }, [groups, selectedDictCode]);

  async function refreshRows() {
    await groupsQuery.refetch();
    setLastEvent(`${meta.title} 已刷新`);
  }

  function openCreateDialog() {
    setEditingItemCode(null);
    setItemForm(emptyItemForm);
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
      const itemCode = requiredText(itemForm.itemCode, "item_code");
      const request: UpsertSystemDictionaryItemRequest = {
        item_name: requiredText(itemForm.itemName, "item_name"),
        owner_id: nullableText(itemForm.ownerId),
        enabled: itemForm.enabled,
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
        itemCode: requiredText(disableForm.itemCode, "item_code"),
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
    <section className="mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 py-8 xl:px-6">
      <PageHeader
        title={meta.title}
        subtitle={meta.subtitle}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {lastEvent && (
              <span className="text-sm text-muted-foreground" role="status">
                {lastEvent}
              </span>
            )}
            <Button type="button" variant="outline" onClick={refreshRows}>
              <RefreshCw className="size-4" aria-hidden />
              刷新
            </Button>
            <Button type="button" variant="outline" onClick={onBack}>
              <ArrowLeft className="size-4" aria-hidden />
              返回工作台
            </Button>
          </div>
        }
      />

      <div className="grid gap-3 md:grid-cols-3">
        <Metric label="字典分类" value={groups.length} />
        <Metric label="启用项" value={activeCount} />
        <Metric label="API 返回" value={groupsQuery.data ? totalCount : 0} />
      </div>

      {groupsQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {groupsQuery.error.message}
        </div>
      )}

      <SystemDictionaryTwoPane
        groups={groups}
        selectedGroupCode={activeGroup.code}
        onSelectedGroupCodeChange={setSelectedDictCode}
        headerActions={
          <Button type="button" size="sm" onClick={openCreateDialog}>
            <Plus className="size-4" aria-hidden />
            新增
          </Button>
        }
        renderItemActions={(item) => {
          const actionItem = activeGroup.items.find(
            (candidate) => candidate.code === item.code && candidate.source === item.source,
          );
          if (!actionItem) return null;
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
        }}
        emptyTitle={meta.emptyTitle}
        emptyDescription={
          groupsQuery.isPending
            ? "正在读取系统字典项。"
            : `${activeGroup.name} 暂无可展示字典项。`
        }
      />

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
              label="item_code"
              value={itemForm.itemCode}
              readOnly={editing}
              onChange={(itemCode) => onItemFormChange((value) => ({ ...value, itemCode }))}
            />
            <TextField
              label="item_name"
              value={itemForm.itemName}
              onChange={(itemName) => onItemFormChange((value) => ({ ...value, itemName }))}
            />
            <TextField
              label="owner_id"
              value={itemForm.ownerId}
              onChange={(ownerId) => onItemFormChange((value) => ({ ...value, ownerId }))}
            />
            <div className="flex items-center gap-2 self-end rounded-md border px-3 py-2">
              <Checkbox
                id="dictionary-item-enabled"
                checked={itemForm.enabled}
                onCheckedChange={(checked) =>
                  onItemFormChange((value) => ({ ...value, enabled: checked === true }))
                }
              />
              <Label htmlFor="dictionary-item-enabled">enabled</Label>
            </div>
            <TextField
              label="effective_from"
              type="datetime-local"
              value={itemForm.effectiveFrom}
              onChange={(effectiveFrom) =>
                onItemFormChange((value) => ({ ...value, effectiveFrom }))
              }
            />
            <TextField
              label="effective_to"
              type="datetime-local"
              value={itemForm.effectiveTo}
              onChange={(effectiveTo) => onItemFormChange((value) => ({ ...value, effectiveTo }))}
            />
            <label className="grid gap-1 text-xs text-muted-foreground md:col-span-2">
              params JSON
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
              label="owner_id"
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

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <Card className="rounded-lg shadow-sm">
      <CardContent className="p-4">
        <p className="text-xs font-medium text-muted-foreground">{label}</p>
        <p className="mt-2 text-2xl font-semibold tracking-normal text-foreground">{value}</p>
      </CardContent>
    </Card>
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

function parseJsonObject(value: string): Record<string, unknown> {
  const trimmed = value.trim();
  if (!trimmed) return {};
  const parsed: unknown = JSON.parse(trimmed);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("params 必须是 JSON 对象");
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
