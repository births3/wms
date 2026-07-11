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
  middlePackage: string;
  largePackage: string;
  unitLengthMm: string;
  unitWidthMm: string;
  unitHeightMm: string;
  unitWeightG: string;
  unitVolumeCm3: string;
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
  const attrs = objectCopy(row.productFields?.attrs);
  return {
    id: row.id,
    productCode: row.code,
    productName: row.name,
    spec: cleanText(row.productFields?.spec ?? row.primaryValue),
    approvalNo: cleanText(row.productFields?.approvalNo ?? row.secondaryValue),
    dosageForm: cleanText(row.productFields?.dosageForm),
    manufacturer: cleanText(row.productFields?.manufacturer),
    specialDrugCategoryCode: cleanText(row.productFields?.specialDrugCategoryCode),
    storageCondition: storageConditionCode(
      row.productFields?.storageCondition ?? row.extraValue,
    ),
    status: row.status || "active",
    middlePackage: cleanText(row.productFields?.middlePackage ?? attrText(attrs, "middle_package")),
    largePackage: cleanText(row.productFields?.largePackage ?? attrText(attrs, "large_package")),
    unitLengthMm: cleanText(row.productFields?.unitLengthMm ?? attrText(attrs, "unit_length_mm")),
    unitWidthMm: cleanText(row.productFields?.unitWidthMm ?? attrText(attrs, "unit_width_mm")),
    unitHeightMm: cleanText(row.productFields?.unitHeightMm ?? attrText(attrs, "unit_height_mm")),
    unitWeightG: cleanText(row.productFields?.unitWeightG ?? attrText(attrs, "unit_weight_g")),
    unitVolumeCm3: cleanText(row.productFields?.unitVolumeCm3 ?? attrText(attrs, "unit_volume_cm3")),
    attrs,
  };
}

export function productEditRequestFromForm(form: ProductEditFormState): UpdateProductRequest {
  const attrs = {
    ...objectCopy(form.attrs),
    storage_condition: form.storageCondition,
  };
  setTextAttr(attrs, "middle_package", form.middlePackage);
  setTextAttr(attrs, "large_package", form.largePackage);
  setTextAttr(attrs, "unit_length_mm", form.unitLengthMm);
  setTextAttr(attrs, "unit_width_mm", form.unitWidthMm);
  setTextAttr(attrs, "unit_height_mm", form.unitHeightMm);
  setTextAttr(attrs, "unit_weight_g", form.unitWeightG);
  setTextAttr(attrs, "unit_volume_cm3", form.unitVolumeCm3);

  return {
    product_name: requiredText(form.productName),
    spec: nullableText(form.spec),
    approval_no: nullableText(form.approvalNo),
    dosage_form: nullableText(form.dosageForm),
    manufacturer: nullableText(form.manufacturer),
    special_drug_category_code: nullableText(form.specialDrugCategoryCode),
    status: form.status,
    attrs,
  };
}

/** 表单绑定用英文 code；兼容列表已中文化的 extraValue 回退 */
function storageConditionCode(value: unknown): string {
  const raw = cleanText(value);
  if (!raw) return "normal";
  const normalized = raw.toLowerCase();
  if (["frozen", "freeze", "冷冻"].includes(normalized)) return "frozen";
  if (["cold", "refrigerated", "冷藏"].includes(normalized)) return "cold";
  if (["cool", "cool_storage", "阴凉"].includes(normalized)) return "cool";
  if (["normal", "ambient", "room", "常温"].includes(normalized)) return "normal";
  return raw;
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

function attrText(attrs: Record<string, unknown>, key: string) {
  const value = attrs[key];
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  return "";
}

function setTextAttr(attrs: Record<string, unknown>, key: string, value: string) {
  const normalized = value.trim();
  attrs[key] = normalized ? normalized : null;
}
