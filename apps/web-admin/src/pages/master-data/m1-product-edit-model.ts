import type { UpdateProductRequest } from "@/features/master-data/master-data-queries";
import type { MasterDataRow } from "@/features/master-data/master-data-queries";

export interface ProductEditFormState {
  id: string;
  productCode: string;
  productName: string;
  spec: string;
  approvalNo: string;
  dosageForm: string;
  manufacturer: string;
  specialDrugCategoryCode: string;
  storageCondition: string;
  status: string;
  attrs: Record<string, unknown>;
}

export const productStorageConditionOptions = [
  { value: "frozen", label: "冷冻" },
  { value: "cold", label: "冷藏" },
  { value: "cool", label: "阴凉" },
  { value: "normal", label: "常温" },
] as const;

export const productStatusOptions = [
  { value: "active", label: "启用" },
  { value: "disabled", label: "停用" },
  { value: "pending_mapping", label: "待映射" },
] as const;

export function productEditFormFromRow(row: MasterDataRow): ProductEditFormState {
  return {
    id: row.id,
    productCode: row.code,
    productName: row.name,
    spec: cleanText(row.productFields?.spec ?? row.primaryValue),
    approvalNo: cleanText(row.productFields?.approvalNo ?? row.secondaryValue),
    dosageForm: cleanText(row.productFields?.dosageForm),
    manufacturer: cleanText(row.productFields?.manufacturer),
    specialDrugCategoryCode: cleanText(row.productFields?.specialDrugCategoryCode),
    storageCondition: cleanText(row.productFields?.storageCondition ?? row.extraValue) || "normal",
    status: row.status || "active",
    attrs: objectCopy(row.productFields?.attrs),
  };
}

export function productEditRequestFromForm(form: ProductEditFormState): UpdateProductRequest {
  return {
    product_name: requiredText(form.productName),
    spec: nullableText(form.spec),
    approval_no: nullableText(form.approvalNo),
    dosage_form: nullableText(form.dosageForm),
    manufacturer: nullableText(form.manufacturer),
    special_drug_category_code: nullableText(form.specialDrugCategoryCode),
    status: form.status,
    attrs: {
      ...objectCopy(form.attrs),
      storage_condition: form.storageCondition,
    },
  };
}

function requiredText(value: string) {
  return value.trim();
}

function nullableText(value: string) {
  const normalized = value.trim();
  return normalized ? normalized : null;
}

function cleanText(value: unknown) {
  if (typeof value !== "string") return "";
  const normalized = value.trim();
  return normalized === "-" ? "" : normalized;
}

function objectCopy(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return { ...(value as Record<string, unknown>) };
}
