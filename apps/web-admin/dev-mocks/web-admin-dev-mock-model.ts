

export const devMockEnabled = process.env.WMS_WEB_ADMIN_DEV_MOCK === "1";
export const devOwnerId = "00000000-0000-0000-0000-000000000001";
export const devUserId = "00000000-0000-0000-0000-000000000101";
export const devWarehouseId = "00000000-0000-0000-0000-000000003001";
export const devLocationId = "00000000-0000-0000-0000-000000000201";
export const devSeedOrderCount = 100;
export const devLoginPassword = ["Correct", "Horse1!"].join("");

export function devLoginDefaults(enabled: boolean) {
  return enabled
    ? {
        enabled: true,
        ownerCode: "PY_OWNER",
        username: "admin",
        password: devLoginPassword,
      }
    : {
        enabled: false,
        ownerCode: "",
        username: "",
        password: "",
      };
}

export interface DevOrderLine {
  line_no: number;
  product_code: string;
  product_id: string | null;
  batch_no: string | null;
  expected_qty: number;
  production_date: string | null;
  expiry_date: string | null;
}

export interface DevOrder {
  id: string;
  owner_id: string;
  receipt_no: string;
  document_type: "purchase_inbound" | "sales_return";
  warehouse_id: string;
  status: string;
  expected_arrival_at: string | null;
  external_ref: string | null;
  supplier_id: string | null;
  created_at: string;
  updated_at: string;
  lines: DevOrderLine[];
}

export interface DevProduct {
  id: string;
  owner_id: string;
  product_code: string;
  product_name: string;
  spec: string | null;
  dosage_form: string | null;
  approval_no: string | null;
  manufacturer: string | null;
  special_drug_category_code: string | null;
  attrs: Record<string, unknown>;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DevSupplier {
  id: string;
  owner_id: string;
  supplier_code: string;
  supplier_name: string;
  license_no: string | null;
  contact_name: string | null;
  source?: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DevCustomer {
  id: string;
  owner_id: string;
  customer_code: string;
  customer_name: string;
  license_no: string | null;
  source?: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DevWarehouse {
  id: string;
  owner_id: string;
  warehouse_code: string;
  warehouse_name: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DevLocation {
  id: string;
  owner_id: string;
  warehouse_id: string;
  zone_id: string;
  location_code: string;
  row_no: number;
  column_no: number;
  layer_no: number;
  max_volume_cm3: number;
  used_volume_cm3: number;
  max_sku_count: number;
  location_type: string;
  bound_owner_id: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface DevInventoryBatch {
  id: string;
  owner_id: string;
  product_code: string;
  batch_no: string;
  production_date: string;
  expiry_date: string;
  qty_on_hand: number;
  qty_locked: number;
  quality_status: string;
  location_id: string;
  location_code: string;
  recall_flag: boolean;
  created_at: string;
  updated_at: string;
}

export interface DevSystemDictionaryItem {
  id: string;
  dict_code: string;
  item_code: string;
  item_name: string;
  owner_id: string | null;
  params: Record<string, unknown>;
  source: string;
  enabled: boolean;
  effective_from: string | null;
  effective_to: string | null;
  disabled_reason: string | null;
  created_at: string;
  updated_at: string;
}

export interface DevPrintTemplate {
  id: string;
  template_code: string;
  template_name: string;
  template_type_code: string;
  owner_id: string;
  scope: "global" | "owner";
  enabled: boolean;
  is_default: boolean;
  remark: string | null;
  latest_version_id: string;
  latest_version_no: number;
  latest_version_status: string;
  field_library_version_id: string;
  designer_version: string;
  created_at: string;
  updated_at: string;
  published_at: string | null;
  hiprint_json: Record<string, unknown>;
  field_bindings: Array<{ field_path: string; required: boolean }>;
  paper: Record<string, unknown>;
}

export interface DevFeatureFlagConfig {
  key: string;
  owner: string;
  created_at: string;
  cleanup_by: string;
  enabled: boolean;
  source: string;
}

function devSystemDictionaryItem(
  id: string,
  dictCode: string,
  itemCode: string,
  itemName: string,
  params: Record<string, unknown>,
): DevSystemDictionaryItem {
  return {
    id,
    dict_code: dictCode,
    item_code: itemCode,
    item_name: itemName,
    owner_id: null,
    params,
    source: "global",
    enabled: true,
    effective_from: null,
    effective_to: null,
    disabled_reason: null,
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  };
}

export const devCreatedOrders: DevOrder[] = [];
export const devCreatedProducts: DevProduct[] = [];
export const devCreatedSuppliers: DevSupplier[] = [];
export const devCreatedCustomers: DevCustomer[] = [];
export const devCreatedWarehouses: DevWarehouse[] = [];
export const devCreatedLocations: DevLocation[] = [];
export const devCreatedPrintTemplates: DevPrintTemplate[] = [];
export const devPrintTemplateVersions = new Map<string, DevPrintTemplate[]>();
export const devSeedOrderStatusOverrides = new Map<string, string>();

export const devUser = {
  user_id: devUserId,
  owner_id: devOwnerId,
  owner_code: "PY_OWNER",
  username: "admin",
  display_name: "Test Admin",
  roles: ["admin", "receiving"],
  permissions: [
    "h1.auth.me",
    "h1.menu.read",
    "h1.menu.write",
    "h1.menu.publish",
    "h4.notify.read",
    "h4.notify.write",
    "h4.notify.send",
    "h4.approval.write",
    "h5.express.read",
    "h5.express.write",
    "h9.print_template.read",
    "h9.print_template.write",
    "h9.print_template.publish",
    "h9.print_template.print",
    "m2.receive",
    "m2.inspect",
    "m2.sign",
    "m2.putaway",
  ],
};

export let devProduct: DevProduct = {
  id: "00000000-0000-0000-0000-000000001001",
  owner_id: devOwnerId,
  product_code: "P-M1-001",
  product_name: "冷藏胰岛素注射液",
  spec: "10ml*1支",
  dosage_form: "注射剂",
  approval_no: "国药准字H20260001",
  manufacturer: "鹏鹞示例药业",
  special_drug_category_code: "none",
  attrs: {
    large_package: "20 件/大包",
    middle_package: "10 件/中包",
    source: "api_import",
    storage_condition: "cold",
    unit_height_mm: "30",
    unit_length_mm: "120",
    unit_volume_cm3: "360",
    unit_weight_g: "180",
    unit_width_mm: "100",
  } as Record<string, unknown>,
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export const devSeedProducts: DevProduct[] = [
  devProduct,
  {
    id: "00000000-0000-0000-0000-000000001002",
    owner_id: devOwnerId,
    product_code: "P-M1-002",
    product_name: "批量导入感冒灵颗粒",
    spec: "10g*9袋",
    dosage_form: "颗粒剂",
    approval_no: "国药准字Z20260002",
    manufacturer: "鹏鹞示例药业",
    special_drug_category_code: "none",
    attrs: {
      large_package: "24 盒/大包",
      middle_package: "12 盒/中包",
      source: "batch_import",
      storage_condition: "normal",
      unit_height_mm: "55",
      unit_length_mm: "150",
      unit_volume_cm3: "990",
      unit_weight_g: "240",
      unit_width_mm: "120",
    },
    status: "active",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  },
  {
    id: "00000000-0000-0000-0000-000000001003",
    owner_id: devOwnerId,
    product_code: "P-M1-003",
    product_name: "手工新建维生素片",
    spec: "100片/瓶",
    dosage_form: "片剂",
    approval_no: "国药准字H20260003",
    manufacturer: "鹏鹞示例药业",
    special_drug_category_code: "none",
    attrs: {
      large_package: "30 瓶/大包",
      middle_package: "10 瓶/中包",
      source: "manual",
      storage_condition: "normal",
      unit_height_mm: "90",
      unit_length_mm: "60",
      unit_volume_cm3: "324",
      unit_weight_g: "120",
      unit_width_mm: "60",
    },
    status: "active",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  },
];

export let devSupplier: DevSupplier = {
  id: "00000000-0000-0000-0000-000000001101",
  owner_id: devOwnerId,
  supplier_code: "S-M1-001",
  supplier_name: "鹏鹞示例供应商",
  license_no: "SPL-2026-001",
  contact_name: "王供应",
  source: "api_import",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devCustomer: DevCustomer = {
  id: "00000000-0000-0000-0000-000000001201",
  owner_id: devOwnerId,
  customer_code: "C-M1-001",
  customer_name: "鹏鹞示例门店",
  license_no: "CPL-2026-001",
  source: "api_import",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devWarehouse: DevWarehouse = {
  id: devWarehouseId,
  owner_id: devOwnerId,
  warehouse_code: "WH-M1-001",
  warehouse_name: "鹏鹞冷链仓",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export let devLocation: DevLocation = {
  id: devLocationId,
  owner_id: devOwnerId,
  warehouse_id: devWarehouseId,
  zone_id: "00000000-0000-0000-0000-000000003101",
  location_code: "A01-01-02-03",
  row_no: 1,
  column_no: 2,
  layer_no: 3,
  max_volume_cm3: 1000000,
  used_volume_cm3: 120000,
  max_sku_count: 3,
  location_type: "storage",
  bound_owner_id: null,
  status: "available",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

export const devSystemDictionaryItemsByCode: Record<string, DevSystemDictionaryItem[]> = {
  document_type: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001501", "document_type", "purchase_inbound", "采购入库", {
      batch_policy: "standard_batch",
      direction: "inbound",
      workflow_template: "purchase_inbound",
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001502", "document_type", "sales_return", "销售退货入库", {
      batch_policy: "standard_batch",
      direction: "inbound",
      workflow_template: "sales_return",
    }),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001503",
      "document_type",
      "purchase_return_outbound",
      "采购退货出库",
      {
        batch_policy: "standard_batch",
        direction: "outbound",
        workflow_template: "purchase_return_outbound",
      },
    ),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001504", "document_type", "sales_outbound", "销售出库", {
      batch_policy: "standard_batch",
      direction: "outbound",
      workflow_template: "sales_outbound",
    }),
  ],
  print_template_type: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001801", "print_template_type", "asn", "ASN 单", {
      business_direction: "inbound",
      business_module: "M2",
      default_scope: "global",
      field_library_code: "m2_asn",
      paper_type: "a4",
    }),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001802",
      "print_template_type",
      "acceptance_record",
      "验收记录单",
      {
        business_direction: "inbound",
        business_module: "M2",
        default_scope: "global",
        field_library_code: "m2_acceptance_record",
        paper_type: "a4",
      },
    ),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001803",
      "print_template_type",
      "delivery_note",
      "随货同行单",
      {
        business_direction: "outbound",
        business_module: "M4",
        default_scope: "global",
        field_library_code: "m4_delivery_note",
        paper_type: "a4",
      },
    ),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001804",
      "print_template_type",
      "location_label",
      "库位标签",
      {
        business_direction: "label",
        business_module: "M1",
        default_scope: "global",
        field_library_code: "m1_location_label",
        paper_type: "label",
      },
    ),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001805", "print_template_type", "lpn_label", "LPN 标签", {
      business_direction: "label",
      business_module: "M3",
      default_scope: "global",
      field_library_code: "m3_lpn_label",
      paper_type: "label",
    }),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001806",
      "print_template_type",
      "product_label",
      "商品标签",
      {
        business_direction: "label",
        business_module: "M1",
        default_scope: "global",
        field_library_code: "m1_product_label",
        paper_type: "label",
      },
    ),
  ],
  special_drug_category: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001601", "special_drug_category", "none", "普通药品", {
      requires_dual_sign: false,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001602", "special_drug_category", "narcotic", "麻醉药品", {
      requires_dual_sign: true,
    }),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001603",
      "special_drug_category",
      "psychotropic_1",
      "第一类精神药品",
      { requires_dual_sign: true },
    ),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001604",
      "special_drug_category",
      "psychotropic_2",
      "第二类精神药品",
      { requires_dual_sign: true },
    ),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001605",
      "special_drug_category",
      "toxic_medical",
      "医疗用毒性药品",
      { requires_dual_sign: true },
    ),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001606", "special_drug_category", "radioactive", "放射性药品", {
      requires_dual_sign: true,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001607", "special_drug_category", "vaccine", "疫苗", {
      requires_dual_sign: true,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001608", "special_drug_category", "blood_product", "血液制品", {
      requires_dual_sign: true,
    }),
  ],
  temperature_zone: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001701", "temperature_zone", "normal", "常温", {
      max_celsius: 30,
      min_celsius: 10,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001702", "temperature_zone", "cool", "阴凉", {
      max_celsius: 20,
      min_celsius: 0,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001703", "temperature_zone", "cold", "冷藏", {
      max_celsius: 8,
      min_celsius: 2,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001704", "temperature_zone", "frozen", "冷冻", {
      max_celsius: -10,
    }),
  ],
  quality_color: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001711", "quality_color", "qualified_green", "合格绿", {
      inventory_quality_status: "qualified",
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001712", "quality_color", "quarantine_yellow", "待验黄", {
      inventory_quality_status: "quarantine",
    }),
    devSystemDictionaryItem(
      "00000000-0000-0000-0000-000000001713",
      "quality_color",
      "unqualified_red",
      "不合格红",
      { inventory_quality_status: "unqualified" },
    ),
  ],
  zone_type: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001721", "zone_type", "storage", "存储区", {
      allow_stock: true,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001722", "zone_type", "receiving", "待验区", {
      allow_stock: false,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001723", "zone_type", "return", "退货区", {
      allow_stock: true,
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001724", "zone_type", "unqualified", "不合格区", {
      allow_stock: true,
      quality_color: "unqualified_red",
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001725", "zone_type", "shipping", "发货暂存区", {
      allow_stock: false,
    }),
  ],
  location_type: [
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001731", "location_type", "storage", "存储位", {
      picking_mode: "none",
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001732", "location_type", "case_pick", "箱拣位", {
      picking_mode: "case",
    }),
    devSystemDictionaryItem("00000000-0000-0000-0000-000000001733", "location_type", "piece_pick", "零拣位", {
      picking_mode: "piece",
    }),
  ],
};

export let devFeatureFlagSource = "config_center";
export let devFeatureFlags: DevFeatureFlagConfig[] = [
  {
    key: "m1.master_data_crud",
    owner: "M1",
    created_at: "2026-06-29",
    cleanup_by: "2026-08-31",
    enabled: true,
    source: "config_center",
  },
  {
    key: "m1.special_drug_category",
    owner: "M1",
    created_at: "2026-06-29",
    cleanup_by: "2026-08-31",
    enabled: true,
    source: "config_center",
  },
];
