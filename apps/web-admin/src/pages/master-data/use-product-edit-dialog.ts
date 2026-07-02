import * as React from "react";

import {
  useUpdateProductMutation,
  type MasterDataRow,
} from "@/features/master-data/master-data-queries";
import {
  productEditFormFromRow,
  productEditRequestFromForm,
  type ProductEditFormState,
} from "./m1-product-edit-model";

interface UseProductEditDialogOptions {
  refetchRows: () => Promise<unknown>;
  onSaved: (productCode: string) => void;
}

export function useProductEditDialog({ refetchRows, onSaved }: UseProductEditDialogOptions) {
  const updateProductMutation = useUpdateProductMutation();
  const [form, setForm] = React.useState<ProductEditFormState | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  function openDialog(row: MasterDataRow) {
    setForm(productEditFormFromRow(row));
    setError(null);
  }

  function closeDialog(open: boolean) {
    if (open) return;
    setForm(null);
    setError(null);
  }

  function updateForm(patch: Partial<ProductEditFormState>) {
    setForm((current) => (current ? { ...current, ...patch } : current));
    setError(null);
  }

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!form) return;
    setError(null);
    try {
      const saved = await updateProductMutation.mutateAsync({
        id: form.id,
        request: productEditRequestFromForm(form),
      });
      await refetchRows();
      setForm(null);
      onSaved(saved.product_code);
    } catch (errorValue) {
      setError(errorValue instanceof Error ? errorValue.message : "保存商品档案失败");
    }
  }

  return {
    form,
    pending: updateProductMutation.isPending,
    error,
    openDialog,
    closeDialog,
    updateForm,
    submit,
  };
}
