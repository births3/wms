import {
  type Customer,
  type Location,
  type MasterDataRow,
  type Product,
  type SpecialDrugCategoryOption,
  type SystemDictionaryItem,
  type SystemDictionaryPaneItem,
  type Supplier,
  type Warehouse,
  type WarehouseZone,
  type WarehouseRef,
} from "./types";

export function warehouseZoneRow(item: WarehouseZone, warehouses: ReadonlyMap<string, WarehouseRef>): MasterDataRow {
  const warehouse = warehouses.get(item.warehouse_id);
  const warehouseLabel = warehouse ? `${warehouse.code} · ${warehouse.name}` : shortId(item.warehouse_id);
  return row({
    id: item.id,
    code: item.zone_code,
    name: item.zone_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "仓库",
    primaryValue: warehouseLabel,
    secondaryLabel: "温区",
    secondaryValue: item.temperature_zone,
    extraLabel: "色标",
    extraValue: item.quality_color,
    createdAt: item.created_at,
    updatedAt: item.updated_at,
    zoneFields: {
      owner: item.owner_id,
      warehouse: warehouseLabel,
      warehouseId: item.warehouse_id,
      zone: item.zone_name,
      zoneId: item.id,
      locationCount: "-",
      availableLocationCount: "-",
    },
  });
}

export function productRow(item: Product): MasterDataRow {
  const storageConditionCode = text(item.attrs.storage_condition);
  const storageConditionLabel = storageConditionDisplayLabel(storageConditionCode);
  const sourceValue = productSourceLabel(item.attrs.source);
  const middlePackage = productAttrText(item.attrs, "middle_package");
  const largePackage = productAttrText(item.attrs, "large_package");
  const unitLengthMm = productAttrText(item.attrs, "unit_length_mm");
  const unitWidthMm = productAttrText(item.attrs, "unit_width_mm");
  const unitHeightMm = productAttrText(item.attrs, "unit_height_mm");
  const unitWeightG = productAttrText(item.attrs, "unit_weight_g");
  const unitVolumeCm3 = productAttrText(item.attrs, "unit_volume_cm3");
  return row({
    id: item.id,
    code: item.product_code,
    name: item.product_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "规格",
    primaryValue: text(item.spec),
    secondaryLabel: "批准文号",
    secondaryValue: text(item.approval_no),
    extraLabel: "储存条件",
    extraValue: storageConditionLabel,
    createdAt: item.created_at,
    sourceValue,
    updatedAt: item.updated_at,
    productFields: {
      approvalNo: item.approval_no,
      attrs: item.attrs,
      dosageForm: item.dosage_form,
      manufacturer: item.manufacturer,
      specialDrugCategoryCode: item.special_drug_category_code,
      spec: item.spec,
      storageCondition: storageConditionCode === "-" ? null : storageConditionCode,
      middlePackage,
      largePackage,
      unitLengthMm,
      unitWidthMm,
      unitHeightMm,
      unitWeightG,
      unitVolumeCm3,
    },
  });
}

export function supplierRow(item: Supplier): MasterDataRow {
  return row({
    id: item.id,
    code: item.supplier_code,
    name: item.supplier_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "统一社会信用代码",
    primaryValue: text(item.license_no),
    secondaryLabel: "联系人",
    secondaryValue: text(item.contact_name),
    extraLabel: "档案类型",
    extraValue: "供应商",
    createdAt: item.created_at,
    sourceValue: supplierSource(item),
    updatedAt: item.updated_at,
    partnerKind: "supplier",
    partnerTypeLabel: "供应商",
  });
}

export function customerRow(item: Customer): MasterDataRow {
  return row({
    id: item.id,
    code: item.customer_code,
    name: item.customer_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "资质证号",
    primaryValue: text(item.license_no),
    secondaryLabel: "档案类型",
    secondaryValue: "客户/门店",
    extraLabel: "货主",
    extraValue: shortId(item.owner_id),
    createdAt: item.created_at,
    sourceValue: customerSource(item),
    updatedAt: item.updated_at,
    partnerKind: "customer",
    partnerTypeLabel: "客户/门店",
  });
}

export function warehouseRow(item: Warehouse): MasterDataRow {
  const warehouseTypeLabel = ({ physical: "物理仓", logical: "逻辑仓", virtual: "虚拟仓" } as Record<string, string>)[item.warehouse_type] ?? item.warehouse_type;
  return row({
    id: item.id,
    code: item.warehouse_code,
    name: item.warehouse_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "货主",
    primaryValue: shortId(item.owner_id),
    secondaryLabel: "仓库类型",
    secondaryValue: warehouseTypeLabel,
    extraLabel: "仓库名称",
    extraValue: item.warehouse_name,
    createdAt: item.created_at,
    updatedAt: item.updated_at,
    warehouseFields: { warehouseType: item.warehouse_type },
  });
}

export function warehouseRefFromWarehouse(item: Warehouse): WarehouseRef {
  return {
    id: item.id,
    code: item.warehouse_code,
    name: item.warehouse_name,
  };
}

export function locationRow(
  item: Location,
  locationTypeLabels: ReadonlyMap<string, string>,
  warehouseRefs: ReadonlyMap<string, WarehouseRef> = new Map(),
): MasterDataRow {
  const locationType = locationTypeLabels.get(item.location_type) ?? text(item.location_type);
  const remainingVolumeCm3 = Math.max(item.max_volume_cm3 - item.used_volume_cm3, 0);
  const volume = `${item.used_volume_cm3}/${item.max_volume_cm3} cm³（余 ${remainingVolumeCm3}）`;
  const area = locationAreaCode(item.location_code);
  const warehouse = warehouseDisplayLabel(warehouseRefs.get(item.warehouse_id), item.warehouse_id);
  const zone = zoneDisplayCode(
    area && area !== "-" ? new Set([area]) : new Set(),
    item.zone_id,
  );
  return row({
    id: item.id,
    code: item.location_code,
    name: `${area}-${item.row_no}-${item.column_no}-${item.layer_no}`,
    status: item.status,
    statusLabel: locationStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "库位类型",
    primaryValue: locationType,
    secondaryLabel: "容量",
    secondaryValue: volume,
    extraLabel: "最大 SKU",
    extraValue: String(item.max_sku_count),
    createdAt: item.created_at,
    updatedAt: item.updated_at,
    locationFields: {
      owner: item.owner_id,
      warehouse,
      warehouseId: item.warehouse_id,
      zone,
      zoneId: item.zone_id,
      area,
      rowNo: String(item.row_no),
      columnNo: String(item.column_no),
      layerNo: String(item.layer_no),
      locationType,
      locationTypeCode: item.location_type,
      volume,
      maxVolumeCm3: String(item.max_volume_cm3),
      usedVolumeCm3: String(item.used_volume_cm3),
      remainingVolumeCm3: String(remainingVolumeCm3),
      maxSku: String(item.max_sku_count),
    },
  });
}

export function systemDictionaryRow(item: SystemDictionaryItem): MasterDataRow {
  return row({
    id: item.id,
    code: item.item_code,
    name: item.item_name,
    status: item.enabled ? "active" : "disabled",
    statusLabel: item.enabled ? "启用" : "停用",
    ownerId: item.owner_id ?? "global",
    primaryLabel: "字典分类",
    primaryValue: item.dict_code,
    secondaryLabel: "来源",
    secondaryValue: item.source,
    extraLabel: "参数",
    extraValue: paramsText(item.params),
    createdAt: item.created_at,
    updatedAt: item.updated_at,
  });
}

export function systemDictionaryPaneItem(item: SystemDictionaryItem): SystemDictionaryPaneItem {
  return {
    id: item.id,
    code: item.item_code,
    name: item.item_name,
    source: item.source,
    enabled: item.enabled,
    ownerId: item.owner_id,
    params: item.params,
    effectiveFrom: item.effective_from,
    effectiveTo: item.effective_to,
    disabledReason: item.disabled_reason,
    updatedAt: item.updated_at,
  };
}

export function specialDrugCategoryOptions(
  categories: readonly SpecialDrugCategoryOption[],
  currentValue = "none",
  activeOnly = true,
): SpecialDrugCategoryOption[] {
  const options = categories
    .filter((category) => !activeOnly || category.status === "active" || category.value === currentValue)
    .map((category) => ({ ...category }));
  if (currentValue && !options.some((option) => option.value === currentValue)) {
    options.unshift({
      value: currentValue,
      label: currentValue === "none" ? "普通药品（none）" : currentValue,
      status: "unknown",
      requiresDualSign: false,
    });
  }
  return options;
}

function row(input: Omit<MasterDataRow, "searchText">): MasterDataRow {
  const locationSearchText = input.locationFields ? Object.values(input.locationFields) : [];
  const zoneSearchText = input.zoneFields ? Object.values(input.zoneFields) : [];
  const productSearchText = input.productFields ? Object.values(input.productFields).filter(isSearchTextValue) : [];
  return {
    ...input,
    searchText: [
      input.code,
      input.name,
      input.status,
      input.statusLabel,
      input.ownerId,
      input.primaryValue,
      input.secondaryValue,
      input.extraValue,
      input.createdAt,
      input.sourceValue ?? "",
      input.partnerTypeLabel ?? "",
      ...zoneSearchText,
      ...productSearchText,
      ...locationSearchText,
    ]
      .join(" ")
      .toLowerCase(),
  };
}

function productAttrText(attrs: Record<string, unknown>, key: string) {
  return text(attrs[key]);
}

function isSearchTextValue(value: unknown): value is string | number {
  return typeof value === "string" || typeof value === "number";
}

export function productSourceLabel(value: unknown) {
  if (typeof value !== "string") return "-";
  const normalized = value.trim().toLowerCase();
  if (!normalized) return "-";
  if (["manual", "manual_create", "manual_created", "hand_created", "手工新建"].includes(normalized)) {
    return "手工新建";
  }
  if (["batch_import", "batch", "excel_import", "import", "批量导入"].includes(normalized)) {
    return "批量导入";
  }
  if (["api_import", "api", "erp", "external_api", "接口导入", "api接口导入"].includes(normalized)) {
    return "API接口导入";
  }
  return value.trim();
}

/** 列表展示用储存条件中文；表单仍使用英文 code（cold/normal/...） */
export function storageConditionDisplayLabel(value: unknown): string {
  if (typeof value !== "string") return "-";
  const raw = value.trim();
  if (!raw || raw === "-") return "-";
  const normalized = raw.toLowerCase();
  if (normalized === "frozen") return "冷冻";
  if (normalized === "cold") return "冷藏";
  if (normalized === "cool") return "阴凉";
  if (normalized === "normal") return "常温";
  return raw;
}

export function warehouseDisplayLabel(
  ref: WarehouseRef | undefined,
  warehouseId: string,
): string {
  if (ref?.code && ref.name) return `${ref.code} · ${ref.name}`;
  if (ref?.code) return ref.code;
  if (ref?.name) return ref.name;
  return warehouseId && warehouseId !== "-" ? shortId(warehouseId) : "-";
}

function zoneDisplayCode(
  areas: ReadonlySet<string>,
  zoneId: string,
  fallbackDisplay?: string,
): string {
  if (areas.size === 1) return Array.from(areas)[0] ?? shortId(zoneId);
  if (fallbackDisplay && fallbackDisplay !== "-" && !looksLikeUuid(fallbackDisplay)) {
    return fallbackDisplay;
  }
  if (areas.size > 1) return `多区域`;
  return zoneId && zoneId !== "-" ? `Z-${shortId(zoneId)}` : "-";
}

function looksLikeUuid(value: string) {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value.trim());
}

function supplierSource(item: Supplier) {
  return productSourceLabel(item.source);
}

function customerSource(item: Customer) {
  return productSourceLabel(item.source);
}

function activeStatusLabel(status: string) {
  if (status === "active") return "启用";
  if (status === "disabled" || status === "inactive") return "停用";
  return status || "未知";
}

function locationStatusLabel(status: string) {
  if (status === "available") return "可用";
  if (status === "occupied") return "占用";
  if (status === "locked") return "锁定";
  return activeStatusLabel(status);
}

function locationAreaCode(locationCode: string) {
  return text(locationCode.split("-")[0]);
}

export function idempotencyKey(prefix: string) {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
  return `${prefix}-${random}`;
}

function shortId(value: string) {
  return value.length > 8 ? value.slice(0, 8) : value;
}

function text(value: unknown) {
  if (typeof value === "string" && value.trim()) return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return "-";
}

function paramsText(params: Record<string, unknown>) {
  const entries = Object.entries(params);
  if (entries.length === 0) return "-";
  return entries
    .slice(0, 3)
    .map(([key, value]) => `${key}=${text(value)}`)
    .join(" / ");
}

export function specialDrugCategoryOptionFromDictionaryItem(
  item: SystemDictionaryItem,
): SpecialDrugCategoryOption {
  const status = item.enabled ? "active" : "disabled";
  const disabledSuffix = item.enabled ? "" : "，已停用";
  return {
    value: item.item_code,
    label: `${item.item_name}（${item.item_code}${disabledSuffix}）`,
    status,
    requiresDualSign: item.params.requires_dual_sign === true,
  };
}
