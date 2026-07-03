import * as React from "react";
import {
  Button,
  Checkbox,
  DataGrid,
  Dialog,
  DialogClose,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  PageHeader,
  StatusBadge,
  type DataGridColumn,
  type StatusKey,
} from "@wms/ui";
import { ArrowLeft, Ban, Pencil, Plus, RefreshCw } from "lucide-react";

import {
  useCreateSpecialDrugCategoryMutation,
  useSpecialDrugCategoriesQuery,
  useUpdateSpecialDrugCategoryMutation,
  type SpecialDrugCategory,
} from "@/features/master-data/master-data-queries";

interface SpecialDrugCategoriesPageProps {
  meta: { title: string; subtitle: string; emptyTitle: string };
  onBack: () => void;
}

interface CategoryFormState { id: string; categoryCode: string; categoryName: string; requiresDualSign: boolean; status: string }

const emptyForm: CategoryFormState = { id: "", categoryCode: "", categoryName: "", requiresDualSign: false, status: "active" };

export function SpecialDrugCategoriesPage({ meta, onBack }: SpecialDrugCategoriesPageProps) {
  const categoriesQuery = useSpecialDrugCategoriesQuery();
  const createMutation = useCreateSpecialDrugCategoryMutation();
  const updateMutation = useUpdateSpecialDrugCategoryMutation();
  const [form, setForm] = React.useState<CategoryFormState | null>(null);
  const [message, setMessage] = React.useState<string | null>(null);
  const [dialogError, setDialogError] = React.useState<string | null>(null);
  const rows = categoriesQuery.data ?? [];
  const pending = createMutation.isPending || updateMutation.isPending;
  const columns: DataGridColumn<SpecialDrugCategory>[] = [
    {
      key: "category_code", header: "分类编码", mono: true, width: 220,
      render: (row) => <span className="text-primary">{row.category_code}</span>,
    },
    { key: "category_name", header: "分类名称", width: 260 },
    {
      key: "requires_dual_sign", header: "双人作业", width: 140,
      render: (row) => (row.requires_dual_sign ? "需要" : "不需要"),
    },
    {
      key: "status", header: "状态", width: 130,
      render: (row) => <StatusBadge status={statusKey(row.status)} label={statusLabel(row.status)} size="sm" />,
    },
    { key: "created_at", header: "创建时间", width: 190 },
    {
      key: "actions",
      header: "操作",
      width: 210,
      render: (row) => (
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" size="sm" onClick={() => openEdit(row)}>
            <Pencil className="size-4" aria-hidden />
            编辑
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={row.status !== "active"} onClick={() => disable(row)}>
            <Ban className="size-4" aria-hidden />
            停用
          </Button>
        </div>
      ),
    },
  ];

  function openCreate() {
    setDialogError(null); setForm(emptyForm);
  }

  function openEdit(category: SpecialDrugCategory) {
    setDialogError(null);
    setForm({
      id: category.id,
      categoryCode: category.category_code,
      categoryName: category.category_name,
      requiresDualSign: category.requires_dual_sign,
      status: category.status,
    });
  }

  async function refreshRows() {
    await categoriesQuery.refetch();
    setMessage(`${meta.title} 已刷新`);
  }

  async function disable(category: SpecialDrugCategory) {
    try {
      const saved = await updateMutation.mutateAsync({
        id: category.id,
        request: { status: "disabled" },
      });
      await categoriesQuery.refetch();
      setMessage(`${saved.category_code} 已停用`);
    } catch (errorValue) {
      setMessage(errorValue instanceof Error ? errorValue.message : "停用特殊药品分类失败");
    }
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form) return;
    setDialogError(null);
    try {
      const saved = form.id
        ? await updateMutation.mutateAsync({
            id: form.id,
            request: {
              category_name: requiredText(form.categoryName, "分类名称"),
              requires_dual_sign: form.requiresDualSign,
              status: form.status,
            },
          })
        : await createMutation.mutateAsync({
            category_code: requiredText(form.categoryCode, "分类编码"),
            category_name: requiredText(form.categoryName, "分类名称"),
            requires_dual_sign: form.requiresDualSign,
          });
      await categoriesQuery.refetch();
      setForm(null); setMessage(`${saved.category_code} 已保存`);
    } catch (errorValue) {
      setDialogError(errorValue instanceof Error ? errorValue.message : "保存特殊药品分类失败");
    }
  }

  return (
    <section className="mx-auto flex w-full max-w-[1680px] flex-col gap-5 px-4 py-8 xl:px-6">
      <PageHeader
        title={meta.title}
        subtitle={meta.subtitle}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            {message && <span className="text-sm text-muted-foreground" role="status">{message}</span>}
            <Button type="button" onClick={openCreate}>
              <Plus className="size-4" aria-hidden />
              新增分类
            </Button>
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

      {categoriesQuery.error && (
        <div className="rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {categoriesQuery.error.message}
        </div>
      )}

      <DataGrid
        columns={columns}
        data={rows}
        rowKey={(row) => row.id}
        caption={categoriesQuery.isPending ? "加载特殊药品分类..." : undefined}
        emptyTitle={meta.emptyTitle}
        storageKey="m1-special-drug-categories-datagrid"
      />

      {form && (
        <Dialog open onOpenChange={(open) => !open && setForm(null)}>
          <DialogContent className="sm:max-w-2xl">
            <form className="grid gap-3 md:grid-cols-2" onSubmit={submit}>
              <DialogHeader className="md:col-span-2">
                <DialogTitle>{form.id ? "编辑特殊药品分类" : "新增特殊药品分类"}</DialogTitle>
              </DialogHeader>
              <TextField
                label="分类编码"
                value={form.categoryCode}
                readOnly={Boolean(form.id)}
                onChange={(categoryCode) => setForm((value) => value && { ...value, categoryCode })}
              />
              <TextField
                label="分类名称"
                value={form.categoryName}
                onChange={(categoryName) => setForm((value) => value && { ...value, categoryName })}
              />
              <label className="flex items-center gap-2 self-end rounded-md border px-3 py-2 text-sm">
                <Checkbox
                  checked={form.requiresDualSign}
                  onCheckedChange={(checked) =>
                    setForm((value) => value && { ...value, requiresDualSign: checked === true })
                  }
                />
                需要双人作业
              </label>
              {dialogError && (
                <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2">
                  {dialogError}
                </div>
              )}
              <DialogFooter className="md:col-span-2">
                <DialogClose asChild>
                  <Button type="button" variant="outline" disabled={pending}>取消</Button>
                </DialogClose>
                <Button type="submit" disabled={pending}>
                  <Pencil className="size-4" aria-hidden />
                  {pending ? "保存中..." : "保存"}
                </Button>
              </DialogFooter>
            </form>
          </DialogContent>
        </Dialog>
      )}
    </section>
  );
}

function TextField({ label, value, onChange, readOnly = false }: { label: string; value: string; onChange: (value: string) => void; readOnly?: boolean }) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Input value={value} readOnly={readOnly} required onChange={(event) => onChange(event.target.value)} />
    </label>
  );
}

function requiredText(value: string, field: string) {
  const trimmed = value.trim();
  if (!trimmed) throw new Error(`${field}必填`);
  return trimmed;
}

function statusKey(status: string): StatusKey {
  if (status === "active") return "completed";
  if (status === "disabled" || status === "inactive") return "isolated";
  return "pending";
}

function statusLabel(status: string) {
  if (status === "active") return "启用";
  if (status === "disabled" || status === "inactive") return "停用";
  return status || "未知";
}
