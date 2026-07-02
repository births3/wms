import * as React from "react";
import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wms/ui";

import {
  productStatusOptions,
  productStorageConditionOptions,
  type ProductEditFormState,
} from "./m1-product-edit-model";

interface ProductEditDialogProps {
  form: ProductEditFormState | null;
  pending: boolean;
  error: string | null;
  onFormChange: (patch: Partial<ProductEditFormState>) => void;
  onOpenChange: (open: boolean) => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}

export function ProductEditDialog({
  form,
  pending,
  error,
  onFormChange,
  onOpenChange,
  onSubmit,
}: ProductEditDialogProps) {
  if (!form) return null;

  return (
    <Dialog open={true} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl">
        <form className="grid gap-4" onSubmit={onSubmit}>
          <DialogHeader>
            <DialogTitle>编辑商品</DialogTitle>
            <DialogDescription>
              商品编码由 ERP 给定，WMS 页面编辑时保持只读。
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-4 md:grid-cols-2">
            <TextField
              label="商品编码"
              value={form.productCode}
              disabled
              onChange={() => undefined}
            />
            <TextField
              label="商品名称"
              value={form.productName}
              required
              onChange={(value) => onFormChange({ productName: value })}
            />
            <TextField
              label="规格"
              value={form.spec}
              onChange={(value) => onFormChange({ spec: value })}
            />
            <TextField
              label="批准文号"
              value={form.approvalNo}
              onChange={(value) => onFormChange({ approvalNo: value })}
            />
            <TextField
              label="剂型"
              value={form.dosageForm}
              onChange={(value) => onFormChange({ dosageForm: value })}
            />
            <TextField
              label="生产企业"
              value={form.manufacturer}
              onChange={(value) => onFormChange({ manufacturer: value })}
            />
            <TextField
              label="特殊药品分类"
              value={form.specialDrugCategoryCode}
              onChange={(value) => onFormChange({ specialDrugCategoryCode: value })}
            />
            <SelectField
              label="储存条件"
              value={form.storageCondition}
              options={productStorageConditionOptions}
              onChange={(value) => onFormChange({ storageCondition: value })}
            />
            <SelectField
              label="状态"
              value={form.status}
              options={productStatusOptions}
              onChange={(value) => onFormChange({ status: value })}
            />
          </div>

          {error && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </div>
          )}

          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={pending}>
                取消
              </Button>
            </DialogClose>
            <Button type="submit" disabled={pending || !form.productName.trim()}>
              {pending ? "保存中..." : "保存"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function TextField({
  label,
  value,
  disabled,
  required,
  onChange,
}: {
  label: string;
  value: string;
  disabled?: boolean;
  required?: boolean;
  onChange: (value: string) => void;
}) {
  const id = React.useId();
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        disabled={disabled}
        required={required}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: readonly { value: string; label: string }[];
  onChange: (value: string) => void;
}) {
  const id = React.useId();
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger id={id}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
