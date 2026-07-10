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
} from "./types";

export function warehouseZoneRowsFromLocations(
  locations: readonly MasterDataRow[],
): MasterDataRow[] {
  const zones = new Map<
    string,
    {
      owner: string;
      warehouse: string;
      zone: string;
      locationCount: number;
      availableLocationCount: number;
      createdAt: string;
      updatedAt: string;
    }
  >();

  for (const location of locations) {
    const fields = location.locationFields;
    if (!fields || fields.warehouse === "-" || fields.zone === "-") continue;
    const owner = fields.owner === "-" ? location.ownerId : fields.owner;
    const key = `${owner}:${fields.warehouse}:${fields.zone}`;
    const current = zones.get(key);
    zones.set(key, {
      owner,
      warehouse: fields.warehouse,
      zone: fields.zone,
      locationCount: (current?.locationCount ?? 0) + 1,
      availableLocationCount:
        (current?.availableLocationCount ?? 0) + (location.status === "available" ? 1 : 0),
      createdAt: current ? minText(current.createdAt, location.createdAt) : location.createdAt,
      updatedAt: current ? maxText(current.updatedAt, location.updatedAt) : location.updatedAt,
    });
  }

  return Array.from(zones.values())
    .sort((left, right) =>
      `${left.warehouse}:${left.zone}`.localeCompare(`${right.warehouse}:${right.zone}`, "zh-CN"),
    )
    .map((zone) =>
      row({
        id: `${zone.owner}:${zone.warehouse}:${zone.zone}`,
        code: zone.zone,
        name: `库区 ${shortId(zone.zone)}`,
        status: "derived_readonly",
        statusLabel: "只读派生",
        ownerId: zone.owner,
        primaryLabel: "仓库 ID",
        primaryValue: zone.warehouse,
        secondaryLabel: "库区 ID",
        secondaryValue: zone.zone,
        extraLabel: "库位数",
        extraValue: `${zone.locationCount} 个 / 可用 ${zone.availableLocationCount} 个`,
        createdAt: zone.createdAt,
        updatedAt: zone.updatedAt,
        zoneFields: {
          owner: zone.owner,
          warehouse: zone.warehouse,
          zone: zone.zone,
          locationCount: String(zone.locationCount),
          availableLocationCount: String(zone.availableLocationCount),
        },
      }),
    );
}
export function productRow(item: Product): MasterDataRow {
  const storageCondition = text(item.attrs.storage_condition ?? item.attrs.storage);
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
    extraValue: storageCondition,
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
      storageCondition,
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
    primaryLabel: "资质证号",
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
    extraValue: item.owner_id,
    createdAt: item.created_at,
    sourceValue: customerSource(item),
    updatedAt: item.updated_at,
    partnerKind: "customer",
    partnerTypeLabel: "客户/门店",
  });
}

 export function warehouseRow(item: Warehouse): MasterDataRow {
  return row({
    id: item.id,
    code: item.warehouse_code,
    name: item.warehouse_name,
    status: item.status,
    statusLabel: activeStatusLabel(item.status),
    ownerId: item.owner_id,
    primaryLabel: "仓库 ID",
    primaryValue: item.id,
    secondaryLabel: "货主",
    secondaryValue: item.owner_id,
    extraLabel: "档案类型",
    extraValue: "仓库",
    createdAt: item.created_at,
    updatedAt: item.updated_at,
  });
}

export function locationRow(item: Location, locationTypeLabels: ReadonlyMap<string, string>): MasterDataRow {
  const locationType = locationTypeLabels.get(item.location_type) ?? text(item.location_type);
  const volume = `${item.used_volume_cm3}/${item.max_volume_cm3} cm³`;
  return row({
    id: item.id,
    code: item.location_code,
    name: `${locationAreaCode(item.location_code)}-${item.row_no}-${item.column_no}-${item.layer_no}`,
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
      warehouse: item.warehouse_id,
      zone: item.zone_id,
      area: locationAreaCode(item.location_code),
      rowNo: String(item.row_no),
      columnNo: String(item.column_no),
      layerNo: String(item.layer_no),
      locationType,
      locationTypeCode: item.location_type,
      volume,
      maxVolumeCm3: String(item.max_volume_cm3),
      usedVolumeCm3: String(item.used_volume_cm3),
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

function minText(left: string, right: string) {
  return left <= right ? left : right;
}

function maxText(left: string, right: string) {
  return left >= right ? left : right;
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
