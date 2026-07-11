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
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@wms/ui";
import { Plus } from "lucide-react";

import {
  specialDrugCategoryOptions,
  useSpecialDrugCategoriesQuery,
  type CreateProductRequest,
} from "@/features/master-data/master-data-queries";
import { productStorageConditionOptions } from "./m1-product-edit-model";

interface ProductCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreate: (request: CreateProductRequest) => Promise<void>;
}

interface ProductFormState {
  productCode: string;
  productName: string;
  approvalNo: string;
  spec: string;
  dosageForm: string;
  manufacturer: string;
  specialDrugCategoryCode: string;
  storageCondition: string;
}

const emptyProductForm: ProductFormState = {
  productCode: "",
  productName: "",
  approvalNo: "",
  spec: "",
  dosageForm: "",
  manufacturer: "",
  specialDrugCategoryCode: "none",
  storageCondition: "normal",
};

export function ProductCreateDialog({ open, onOpenChange, onCreate }: ProductCreateDialogProps) {
  const [form, setForm] = React.useState<ProductFormState>(emptyProductForm);
  const [message, setMessage] = React.useState<string | null>(null);
  const [submitting, setSubmitting] = React.useState(false);
  const categoriesQuery = useSpecialDrugCategoriesQuery(open);
  const categoryOptions = specialDrugCategoryOptions(
    categoriesQuery.data ?? [],
    form.specialDrugCategoryCode,
    true,
  );

  function updateForm(patch: Partial<ProductFormState>) {
    setForm((value) => ({ ...value, ...patch }));
    setMessage(null);
  }

  async function submitProduct(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    setMessage(null);
    try {
      await onCreate({
        product_code: requiredText(form.productCode, "商品编码"),
        product_name: requiredText(form.productName, "商品名称"),
        approval_no: nullableText(form.approvalNo),
        spec: nullableText(form.spec),
        dosage_form: nullableText(form.dosageForm),
        manufacturer: nullableText(form.manufacturer),
        special_drug_category_code: nullableText(form.specialDrugCategoryCode),
        attrs: {
          storage_condition: form.storageCondition,
          source: "manual",
        },
      });
      setForm(emptyProductForm);
      onOpenChange(false);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "新建商品失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <form className="grid gap-3 md:grid-cols-2" onSubmit={submitProduct}>
          <DialogHeader className="md:col-span-2">
            <DialogTitle>新建商品</DialogTitle>
            <DialogDescription>录入商品档案基础字段。</DialogDescription>
          </DialogHeader>
          <TextField label="商品编码" required value={form.productCode} onChange={(productCode) => updateForm({ productCode })} />
          <TextField label="商品名称" required value={form.productName} onChange={(productName) => updateForm({ productName })} />
          <TextField label="规格" value={form.spec} onChange={(spec) => updateForm({ spec })} />
          <TextField label="批准文号" value={form.approvalNo} onChange={(approvalNo) => updateForm({ approvalNo })} />
          <TextField label="剂型" value={form.dosageForm} onChange={(dosageForm) => updateForm({ dosageForm })} />
          <TextField label="生产企业" value={form.manufacturer} onChange={(manufacturer) => updateForm({ manufacturer })} />
          <SelectField
            label="储存条件"
            value={form.storageCondition}
            options={productStorageConditionOptions}
            onChange={(storageCondition) => updateForm({ storageCondition })}
          />
          <SelectField
            label="特殊药品分类"
            value={form.specialDrugCategoryCode}
            options={categoryOptions}
            onChange={(specialDrugCategoryCode) => updateForm({ specialDrugCategoryCode })}
          />
          {message && (
            <div className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive md:col-span-2">
              {message}
            </div>
          )}
          <DialogFooter className="md:col-span-2">
            <DialogClose asChild>
              <Button type="button" variant="outline" disabled={submitting}>
                取消
              </Button>
            </DialogClose>
            <Button type="submit" disabled={submitting}>
              <Plus className="size-4" aria-hidden />
              {submitting ? "提交中..." : "新建商品"}
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
  onChange,
  required,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  required?: boolean;
}) {
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Input required={required} value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
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
  return (
    <label className="grid gap-1 text-xs text-muted-foreground">
      {label}
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger><SelectValue /></SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
          ))}
        </SelectContent>
      </Select>
    </label>
  );
}

function requiredText(value: string, field: string) {
  const trimmed = value.trim();
  if (!trimmed) throw new Error(`${field}必填`);
  return trimmed;
}

function nullableText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}
