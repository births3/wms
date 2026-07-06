import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

import { handleAdminMenuDevMock } from "./dev-mocks/admin-menu-dev-mock";

const devMockEnabled = process.env.WMS_WEB_ADMIN_DEV_MOCK === "1";
const e2eApiUrl = process.env.WMS_WEB_ADMIN_E2E_API_URL?.trim();
const devOwnerId = "00000000-0000-0000-0000-000000000001";
const devUserId = "00000000-0000-0000-0000-000000000101";
const devWarehouseId = "00000000-0000-0000-0000-000000003001";
const devLocationId = "00000000-0000-0000-0000-000000000201";
const devSeedOrderCount = 100;
const devLoginPassword = ["Correct", "Horse1!"].join("");

function devLoginDefaults(enabled: boolean) {
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

interface DevOrderLine {
  line_no: number;
  product_code: string;
  product_id: string | null;
  batch_no: string | null;
  expected_qty: number;
  production_date: string | null;
  expiry_date: string | null;
}

interface DevOrder {
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

interface DevProduct {
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

interface DevSupplier {
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

interface DevCustomer {
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

interface DevWarehouse {
  id: string;
  owner_id: string;
  warehouse_code: string;
  warehouse_name: string;
  status: string;
  created_at: string;
  updated_at: string;
}

interface DevLocation {
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

interface DevInventoryBatch {
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

interface DevSystemDictionaryItem {
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

interface DevFeatureFlagConfig {
  key: string;
  owner: string;
  created_at: string;
  cleanup_by: string;
  enabled: boolean;
  source: string;
}

const devCreatedOrders: DevOrder[] = [];
const devCreatedProducts: DevProduct[] = [];
const devCreatedSuppliers: DevSupplier[] = [];
const devCreatedCustomers: DevCustomer[] = [];
const devCreatedWarehouses: DevWarehouse[] = [];
const devCreatedLocations: DevLocation[] = [];
const devSeedOrderStatusOverrides = new Map<string, string>();

const devUser = {
  user_id: devUserId,
  owner_id: devOwnerId,
  owner_code: "PY_OWNER",
  username: "admin",
  display_name: "Test Admin",
  roles: ["admin", "receiving"],
  permissions: ["h1.auth.me", "h1.menu.read", "h1.menu.write", "h1.menu.publish", "m2.receive", "m2.inspect", "m2.sign", "m2.putaway"],
};

let devProduct: DevProduct = {
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

let devSupplier: DevSupplier = {
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

let devCustomer: DevCustomer = {
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

let devWarehouse: DevWarehouse = {
  id: devWarehouseId,
  owner_id: devOwnerId,
  warehouse_code: "WH-M1-001",
  warehouse_name: "鹏鹞冷链仓",
  status: "active",
  created_at: "2026-06-29T00:00:00.000Z",
  updated_at: "2026-06-29T00:00:00.000Z",
};

let devLocation: DevLocation = {
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

const devSystemDictionaryItemsByCode: Record<string, DevSystemDictionaryItem[]> = {
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

let devFeatureFlagSource = "config_center";
let devFeatureFlags: DevFeatureFlagConfig[] = [
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

function webAdminDevMock(): Plugin {
  return {
    name: "wms-web-admin-dev-mock",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!devMockEnabled || !req.url) {
          next();
          return;
        }

        const pathname = new URL(req.url, "http://wms.local").pathname;
        if (!pathname.startsWith("/api/v1/")) {
          next();
          return;
        }

        try {
          const handled = await handleDevMockRequest(req, res, pathname);
          if (!handled) next();
        } catch (error) {
          sendJson(res, 500, {
            code: "DEV_MOCK_ERROR",
            message: error instanceof Error ? error.message : "Dev mock failed",
            trace_id: "dev-mock",
          });
        }
      });
    },
  };
}

async function handleDevMockRequest(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (req.method === "POST" && pathname === "/api/v1/auth/login") {
    const body = await readJsonBody(req);
    const valid =
      body.owner_code === "PY_OWNER" && body.username === "admin" && body.password === devLoginPassword;

    if (!valid) {
      sendJson(res, 401, {
        code: "AUTH_INVALID_CREDENTIALS",
        message: "Login failed",
        trace_id: "dev-mock",
      });
      return true;
    }

    sendJson(res, 200, {
      access_token: `local-dev-${Date.now()}`,
      token_type: "Bearer",
      expires_at: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
      user: devUser,
    });
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/auth/me") {
    sendJson(res, 200, devUser);
    return true;
  }

  if (pathname.startsWith("/api/v1/config-center/feature-flags")) {
    await handleFeatureFlagRequest(req, res, pathname);
    return true;
  }

  if (pathname.startsWith("/api/v1/admin/menus")) {
    await handleAdminMenuDevMock(req, res, pathname);
    return true;
  }

  const productDetail = pathname.match(/^\/api\/v1\/master-data\/products\/([^/]+)$/);
  if (req.method === "PATCH" && productDetail) {
    await handleProductUpdate(req, res, productDetail[1]);
    return true;
  }

  const supplierDetail = pathname.match(/^\/api\/v1\/master-data\/suppliers\/([^/]+)$/);
  if (req.method === "PATCH" && supplierDetail) {
    await handleSupplierUpdate(req, res, supplierDetail[1]);
    return true;
  }

  const customerDetail = pathname.match(/^\/api\/v1\/master-data\/customers\/([^/]+)$/);
  if (req.method === "PATCH" && customerDetail) {
    await handleCustomerUpdate(req, res, customerDetail[1]);
    return true;
  }

  const warehouseDetail = pathname.match(/^\/api\/v1\/master-data\/warehouses\/([^/]+)$/);
  if (req.method === "PATCH" && warehouseDetail) {
    await handleWarehouseUpdate(req, res, warehouseDetail[1]);
    return true;
  }

  const locationDetail = pathname.match(/^\/api\/v1\/master-data\/locations\/([^/]+)$/);
  if (req.method === "PATCH" && locationDetail) {
    await handleLocationUpdate(req, res, locationDetail[1]);
    return true;
  }

  const systemDictionaryDisable = pathname.match(
    /^\/api\/v1\/system-dictionaries\/([^/]+)\/items\/([^/]+)\/disable$/,
  );
  if (req.method === "PATCH" && systemDictionaryDisable) {
    await handleSystemDictionaryDisable(req, res, systemDictionaryDisable[1], systemDictionaryDisable[2]);
    return true;
  }

  const systemDictionaryItem = pathname.match(/^\/api\/v1\/system-dictionaries\/([^/]+)\/items\/([^/]+)$/);
  if (req.method === "PUT" && systemDictionaryItem) {
    await handleSystemDictionaryUpsert(req, res, systemDictionaryItem[1], systemDictionaryItem[2]);
    return true;
  }

  if (req.method === "GET") {
    const masterDataResponse = devMasterDataResponse(pathname);
    if (masterDataResponse) {
      sendJson(res, 200, masterDataResponse);
      return true;
    }
  }

  if (req.method === "POST" && pathname === "/api/v1/master-data/products") {
    const body = await readJsonBody(req);
    const created = devProductFromCreateRequest(body);
    devCreatedProducts.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/master-data/suppliers") {
    const body = await readJsonBody(req);
    const created = devSupplierFromCreateRequest(body);
    devCreatedSuppliers.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/master-data/customers") {
    const body = await readJsonBody(req);
    const created = devCustomerFromCreateRequest(body);
    devCreatedCustomers.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/master-data/warehouses") {
    const body = await readJsonBody(req);
    const created = devWarehouseFromCreateRequest(body);
    devCreatedWarehouses.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/master-data/locations") {
    const body = await readJsonBody(req);
    const created = devLocationFromCreateRequest(body);
    devCreatedLocations.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/inventory/batches") {
    const data = devSeedInventoryBatches();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  if (req.method === "GET" && pathname === "/api/v1/inbound/receiving-orders") {
    const data = allDevOrders();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/inbound/receiving-orders") {
    const body = await readJsonBody(req);
    const created = devOrderFromCreateRequest(body);
    devCreatedOrders.unshift(created);
    sendJson(res, 200, created);
    return true;
  }

  const action = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)\/([^/]+)$/);
  if (req.method === "POST" && action && findDevOrder(action[1])) {
    await handleInboundAction(req, res, action[2], action[1]);
    return true;
  }

  const detail = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)$/);
  if (req.method === "GET" && detail) {
    const order = findDevOrder(detail[1]);
    if (!order) {
      sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Receiving order not found", trace_id: "dev-mock" });
      return true;
    }
    sendJson(res, 200, order);
    return true;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Dev mock route not found",
    trace_id: "dev-mock",
  });
  return true;
}

function devMasterDataResponse(pathname: string): Record<string, unknown> | null {
  const updatedAt = "2026-06-29T00:00:00.000Z";

  if (pathname === "/api/v1/master-data/products") {
    const data = [...devCreatedProducts, ...devSeedProducts(updatedAt)];
    return {
      data,
      page: { count: data.length, next_cursor: null },
      inventory_alert_count: 0,
      pending_receipt_orders: 0,
      returns_this_month: 0,
      signed_orders_last_7_days: 0,
      store_id: null,
    };
  }

  if (pathname === "/api/v1/master-data/suppliers") {
    const data = [...devCreatedSuppliers, ...devSeedSuppliers(updatedAt)];
    return {
      data,
      page: { count: data.length, next_cursor: null },
    };
  }

  if (pathname === "/api/v1/master-data/customers") {
    const data = [...devCreatedCustomers, ...devSeedCustomers(updatedAt)];
    return {
      data,
      page: { count: data.length, next_cursor: null },
    };
  }

  if (pathname === "/api/v1/master-data/warehouses") {
    const data = [...devCreatedWarehouses, devWarehouse];
    return {
      data,
      page: { count: data.length, next_cursor: null },
    };
  }

  if (pathname === "/api/v1/master-data/locations") {
    const data = [...devCreatedLocations, devLocation];
    return {
      data,
      page: { count: data.length, next_cursor: null },
    };
  }

  const systemDictionaryList = pathname.match(/^\/api\/v1\/system-dictionaries\/([^/]+)\/items$/);
  if (systemDictionaryList) {
    const data = devSystemDictionaryItemsByCode[decodeURIComponent(systemDictionaryList[1])];
    if (!data) return null;
    return {
      data,
      page: { count: data.length, next_cursor: null },
    };
  }

  return null;
}

function devSeedProducts(updatedAt: string): DevProduct[] {
  return [
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
      created_at: updatedAt,
      updated_at: updatedAt,
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
      created_at: updatedAt,
      updated_at: updatedAt,
    },
  ];
}

function devSeedSuppliers(_updatedAt: string): DevSupplier[] {
  return [devSupplier];
}

function devSeedCustomers(_updatedAt: string): DevCustomer[] {
  return [devCustomer];
}

function devSeedInventoryBatches(): DevInventoryBatch[] {
  const now = "2026-06-29T00:00:00.000Z";
  return [
    {
      id: "00000000-0000-0000-0000-000000006001",
      owner_id: devOwnerId,
      product_code: "P-M1-001",
      batch_no: "BATCH-M3-202606-01",
      production_date: "2026-01-01",
      expiry_date: "2028-01-01",
      qty_on_hand: 120,
      qty_locked: 10,
      quality_status: "qualified",
      location_id: devLocationId,
      location_code: devLocation.location_code,
      recall_flag: false,
      created_at: now,
      updated_at: now,
    },
    {
      id: "00000000-0000-0000-0000-000000006002",
      owner_id: devOwnerId,
      product_code: "P-M1-002",
      batch_no: "BATCH-M3-202606-02",
      production_date: "2026-02-01",
      expiry_date: "2027-08-01",
      qty_on_hand: 48,
      qty_locked: 48,
      quality_status: "quarantined",
      location_id: devLocationId,
      location_code: devLocation.location_code,
      recall_flag: true,
      created_at: now,
      updated_at: now,
    },
  ];
}

function devProductFromCreateRequest(body: Record<string, unknown>): DevProduct {
  const now = new Date().toISOString();
  const attrs = asRecord(body.attrs);
  return {
    id: `00000000-0000-0000-0000-${String(1900 + devCreatedProducts.length + 1).padStart(12, "0")}`,
    owner_id: devOwnerId,
    product_code: asString(body.product_code, "P-M1-NEW"),
    product_name: asString(body.product_name, "新建商品"),
    spec: asNullableString(body.spec),
    dosage_form: asNullableString(body.dosage_form),
    approval_no: asNullableString(body.approval_no),
    manufacturer: asNullableString(body.manufacturer),
    special_drug_category_code: asNullableString(body.special_drug_category_code),
    attrs: {
      ...attrs,
      storage_condition: asString(attrs.storage_condition, "normal"),
      source: asString(attrs.source, "api_import"),
    },
    status: "active",
    created_at: now,
    updated_at: now,
  };
}

function devSupplierFromCreateRequest(body: Record<string, unknown>): DevSupplier {
  const now = new Date().toISOString();
  return {
    id: `00000000-0000-0000-0000-${String(2100 + devCreatedSuppliers.length + 1).padStart(12, "0")}`,
    owner_id: devOwnerId,
    supplier_code: asString(body.supplier_code, "S-M1-NEW"),
    supplier_name: asString(body.supplier_name, "新建供应商"),
    license_no: asNullableString(body.license_no),
    contact_name: asNullableString(body.contact_name),
    source: asString(body.source, "api_import"),
    status: "active",
    created_at: now,
    updated_at: now,
  };
}

function devCustomerFromCreateRequest(body: Record<string, unknown>): DevCustomer {
  const now = new Date().toISOString();
  return {
    id: `00000000-0000-0000-0000-${String(2200 + devCreatedCustomers.length + 1).padStart(12, "0")}`,
    owner_id: devOwnerId,
    customer_code: asString(body.customer_code, "C-M1-NEW"),
    customer_name: asString(body.customer_name, "新建客户"),
    license_no: asNullableString(body.license_no),
    source: asString(body.source, "api_import"),
    status: "active",
    created_at: now,
    updated_at: now,
  };
}

function devWarehouseFromCreateRequest(body: Record<string, unknown>): DevWarehouse {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    warehouse_code: asString(body.warehouse_code, "WH-M1-NEW"),
    warehouse_name: asString(body.warehouse_name, "新建仓库"),
    status: "active",
    created_at: now,
    updated_at: now,
  };
}

function devLocationFromCreateRequest(body: Record<string, unknown>): DevLocation {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    warehouse_id: asString(body.warehouse_id, devWarehouseId),
    zone_id: asString(body.zone_id, "00000000-0000-0000-0000-000000003101"),
    location_code: asString(body.location_code, "A01-NEW-01-01"),
    row_no: asNumber(body.row_no, 1),
    column_no: asNumber(body.column_no, 1),
    layer_no: asNumber(body.layer_no, 1),
    max_volume_cm3: asNumber(body.max_volume_cm3, 5000000),
    used_volume_cm3: 0,
    max_sku_count: asNumber(body.max_sku_count, 1),
    location_type: asString(body.location_type, "storage"),
    bound_owner_id: asNullableString(body.bound_owner_id),
    status: "available",
    created_at: now,
    updated_at: now,
  };
}

async function handleFeatureFlagRequest(req: IncomingMessage, res: ServerResponse, pathname: string) {
  if (req.method === "GET" && pathname === "/api/v1/config-center/feature-flags/export") {
    sendJson(res, 200, { source: devFeatureFlagSource, flags: devFeatureFlags });
    return;
  }

  if (req.method === "GET" && pathname === "/api/v1/config-center/feature-flags/reconcile") {
    sendJson(res, 200, { matched: devFeatureFlags.length, missing_in_config_center: [], mismatched: [] });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/config-center/feature-flags/migrate") {
    devFeatureFlagSource = "config_center";
    sendJson(res, 200, { source: "file", target: devFeatureFlagSource, migrated_count: devFeatureFlags.length });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/config-center/feature-flags/import") {
    const body = await readJsonBody(req);
    const flags = Array.isArray(body.flags) ? body.flags.map((item) => devFeatureFlagConfig(asRecord(item))) : [];
    devFeatureFlags = flags;
    sendJson(res, 200, { imported_count: flags.length, target: devFeatureFlagSource });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/config-center/feature-flags/source") {
    const body = await readJsonBody(req);
    devFeatureFlagSource = asString(body.source, "config_center");
    sendJson(res, 200, { active_source: devFeatureFlagSource });
    return;
  }

  if (req.method === "POST" && pathname === "/api/v1/config-center/feature-flags/archive-file-source") {
    const body = await readJsonBody(req);
    sendJson(res, 200, {
      archived_source: "file",
      archive_ref: asString(body.archive_ref, "deploy/feature_flags.toml"),
      archived_at: new Date().toISOString(),
    });
    return;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Feature flag dev mock route not found",
    trace_id: "dev-mock",
  });
}

function devFeatureFlagConfig(body: Record<string, unknown>): DevFeatureFlagConfig {
  return {
    key: asString(body.key, "unknown.flag"),
    owner: asString(body.owner, "M1"),
    created_at: asString(body.created_at, new Date().toISOString().slice(0, 10)),
    cleanup_by: asString(body.cleanup_by, "2026-08-31"),
    enabled: asBoolean(body.enabled, false),
    source: asString(body.source, devFeatureFlagSource),
  };
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

async function handleSystemDictionaryUpsert(
  req: IncomingMessage,
  res: ServerResponse,
  dictCode: string,
  itemCode: string,
) {
  const decodedDictCode = decodeURIComponent(dictCode);
  const decodedItemCode = decodeURIComponent(itemCode);
  const items = devSystemDictionaryItemsByCode[decodedDictCode];
  if (!items) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Dictionary not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const now = new Date().toISOString();
  const ownerId = asNullableString(body.owner_id);
  const existingIndex = items.findIndex(
    (item) => item.item_code === decodedItemCode && item.owner_id === ownerId,
  );
  const next: DevSystemDictionaryItem = {
    id: existingIndex >= 0 ? items[existingIndex].id : crypto.randomUUID(),
    dict_code: decodedDictCode,
    item_code: decodedItemCode,
    item_name: asString(body.item_name, decodedItemCode),
    owner_id: ownerId,
    params: asRecord(body.params),
    source: ownerId ? "owner" : "global",
    enabled: asBoolean(body.enabled, true),
    effective_from: asNullableString(body.effective_from),
    effective_to: asNullableString(body.effective_to),
    disabled_reason: existingIndex >= 0 ? items[existingIndex].disabled_reason : null,
    created_at: existingIndex >= 0 ? items[existingIndex].created_at : now,
    updated_at: now,
  };
  if (existingIndex >= 0) items[existingIndex] = next;
  else items.unshift(next);
  sendJson(res, 200, next);
}

async function handleSystemDictionaryDisable(
  req: IncomingMessage,
  res: ServerResponse,
  dictCode: string,
  itemCode: string,
) {
  const decodedDictCode = decodeURIComponent(dictCode);
  const decodedItemCode = decodeURIComponent(itemCode);
  const items = devSystemDictionaryItemsByCode[decodedDictCode];
  if (!items) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Dictionary not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const ownerId = asNullableString(body.owner_id);
  const index = items.findIndex((item) => item.item_code === decodedItemCode && item.owner_id === ownerId);
  if (index < 0) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Dictionary item not found", trace_id: "dev-mock" });
    return;
  }

  items[index] = {
    ...items[index],
    enabled: false,
    disabled_reason: asNullableString(body.disabled_reason),
    updated_at: new Date().toISOString(),
  };
  sendJson(res, 200, items[index]);
}

async function handleProductUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdProductIndex = devCreatedProducts.findIndex((product) => product.id === id);
  const product = id === devProduct.id ? devProduct : devCreatedProducts[createdProductIndex];
  if (!product) {
    sendJson(res, 404, {
      code: "DEV_MOCK_NOT_FOUND",
      message: "Product not found",
      trace_id: "dev-mock",
    });
    return;
  }

  const body = await readJsonBody(req);
  const attrs = asRecord(body.attrs);
  const storageCondition = asString(attrs.storage_condition, asString(product.attrs.storage_condition, "normal"));
  const updatedProduct: DevProduct = {
    ...product,
    product_name: asString(body.product_name, product.product_name),
    spec: asNullableString(body.spec) ?? product.spec,
    dosage_form: asNullableString(body.dosage_form),
    approval_no: asNullableString(body.approval_no),
    manufacturer: asNullableString(body.manufacturer),
    special_drug_category_code: asNullableString(body.special_drug_category_code),
    attrs: { ...product.attrs, ...attrs, storage_condition: storageCondition },
    status: asString(body.status, product.status),
    updated_at: new Date().toISOString(),
  };

  if (id === devProduct.id) {
    devProduct = updatedProduct;
  } else {
    devCreatedProducts[createdProductIndex] = updatedProduct;
  }
  sendJson(res, 200, updatedProduct);
}

async function handleSupplierUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedSuppliers.findIndex((supplier) => supplier.id === id);
  const supplier = id === devSupplier.id ? devSupplier : devCreatedSuppliers[createdIndex];
  if (!supplier) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Supplier not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const updated: DevSupplier = {
    ...supplier,
    supplier_name: asString(body.supplier_name, supplier.supplier_name),
    license_no: asNullableString(body.license_no) ?? supplier.license_no,
    contact_name: asNullableString(body.contact_name) ?? supplier.contact_name,
    status: asString(body.status, supplier.status),
    updated_at: new Date().toISOString(),
  };
  if (id === devSupplier.id) devSupplier = updated;
  else devCreatedSuppliers[createdIndex] = updated;
  sendJson(res, 200, updated);
}

async function handleCustomerUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedCustomers.findIndex((customer) => customer.id === id);
  const customer = id === devCustomer.id ? devCustomer : devCreatedCustomers[createdIndex];
  if (!customer) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Customer not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const updated: DevCustomer = {
    ...customer,
    customer_name: asString(body.customer_name, customer.customer_name),
    license_no: asNullableString(body.license_no) ?? customer.license_no,
    status: asString(body.status, customer.status),
    updated_at: new Date().toISOString(),
  };
  if (id === devCustomer.id) devCustomer = updated;
  else devCreatedCustomers[createdIndex] = updated;
  sendJson(res, 200, updated);
}

async function handleWarehouseUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedWarehouses.findIndex((warehouse) => warehouse.id === id);
  const warehouse = id === devWarehouse.id ? devWarehouse : devCreatedWarehouses[createdIndex];
  if (!warehouse) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Warehouse not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const updated: DevWarehouse = {
    ...warehouse,
    warehouse_name: asString(body.warehouse_name, warehouse.warehouse_name),
    status: asString(body.status, warehouse.status),
    updated_at: new Date().toISOString(),
  };
  if (id === devWarehouse.id) devWarehouse = updated;
  else devCreatedWarehouses[createdIndex] = updated;
  sendJson(res, 200, updated);
}

async function handleLocationUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedLocations.findIndex((location) => location.id === id);
  const location = id === devLocation.id ? devLocation : devCreatedLocations[createdIndex];
  if (!location) {
    sendJson(res, 404, { code: "DEV_MOCK_NOT_FOUND", message: "Location not found", trace_id: "dev-mock" });
    return;
  }

  const body = await readJsonBody(req);
  const updated: DevLocation = {
    ...location,
    zone_id: asNullableString(body.zone_id) ?? location.zone_id,
    location_code: asString(body.location_code, location.location_code),
    row_no: asNumber(body.row_no, location.row_no),
    column_no: asNumber(body.column_no, location.column_no),
    layer_no: asNumber(body.layer_no, location.layer_no),
    max_volume_cm3: asNumber(body.max_volume_cm3, location.max_volume_cm3),
    used_volume_cm3: asNumber(body.used_volume_cm3, location.used_volume_cm3),
    max_sku_count: asNumber(body.max_sku_count, location.max_sku_count),
    location_type: asString(body.location_type, location.location_type),
    bound_owner_id: asNullableString(body.bound_owner_id) ?? location.bound_owner_id,
    status: asString(body.status, location.status),
    updated_at: new Date().toISOString(),
  };
  if (id === devLocation.id) devLocation = updated;
  else devCreatedLocations[createdIndex] = updated;
  sendJson(res, 200, updated);
}

async function handleInboundAction(req: IncomingMessage, res: ServerResponse, action: string | undefined, orderId: string) {
  const body = await readJsonBody(req);
  const occurredAt = new Date().toISOString();

  if (action === "receive") {
    setDevOrderStatus(orderId, "inspecting");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004001",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: asNumber(body.actual_qty, 120),
      shortage_qty: asNumber(body.shortage_qty, 0),
      rejected_qty: asNumber(body.rejected_qty, 0),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "reject") {
    setDevOrderStatus(orderId, "closed_rejected");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004005",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: 0,
      shortage_qty: 0,
      rejected_qty: devOrderExpectedQty(orderId),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "inspect") {
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004002",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      batch_no: asString(body.batch_no, "BATCH-202606"),
      accepted_qty: asNumber(body.accepted_qty, 120),
      rejected_qty: asNumber(body.rejected_qty, 0),
      quality_status: asString(body.quality_status, "qualified"),
      occurred_at: occurredAt,
    });
    return;
  }

  if (action === "sign") {
    setDevOrderStatus(orderId, "putaway");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004003",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      first_signer_id: asString(body.first_signer_id, devUserId),
      second_signer_id: asNullableString(body.second_signer_id),
      signed_at: occurredAt,
    });
    return;
  }

  if (action === "putaway") {
    setDevOrderStatus(orderId, "completed");
    sendJson(res, 200, {
      id: "00000000-0000-0000-0000-000000004004",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      batch_no: asString(body.batch_no, "BATCH-202606"),
      product_code: asString(body.product_code, "P-M2-001"),
      qty: asNumber(body.qty, 120),
      location_id: asString(body.location_id, devLocationId),
      location_code: asString(body.location_code, "A-01-01"),
      occurred_at: occurredAt,
    });
    return;
  }

  sendJson(res, 404, {
    code: "DEV_MOCK_NOT_FOUND",
    message: "Dev mock route not found",
    trace_id: "dev-mock",
  });
}

function devSeedOrders() {
  return Array.from({ length: devSeedOrderCount }, (_value, index) => devSeedOrder(index + 1));
}

function devSeedOrder(index: number): DevOrder {
  const now = new Date().toISOString();
  const id = devSeedOrderId(index);
  const documentType = devSeedDocumentType(index);
  const isSalesReturn = documentType === "sales_return";
  const padded = String(index).padStart(4, "0");
  const expectedArrivalAt = new Date(Date.UTC(2026, 5, 27 + Math.floor((index - 1) / 24), index % 24, 0, 0)).toISOString();
  return {
    id,
    owner_id: devOwnerId,
    receipt_no: `${isSalesReturn ? "SR" : "ASN"}-M2-PC-${padded}`,
    document_type: documentType,
    warehouse_id: devWarehouseId,
    status: devSeedOrderStatusOverrides.get(id) ?? devSeedOrderStatus(index),
    expected_arrival_at: expectedArrivalAt,
    external_ref: `${isSalesReturn ? "ERP-SR" : "ERP-ASN"}-${padded}`,
    supplier_id: `00000000-0000-0000-0000-${String(5000 + (index % 20)).padStart(12, "0")}`,
    created_at: "2026-06-27T08:00:00.000Z",
    updated_at: now,
    lines: devSeedOrderLines(index, isSalesReturn, padded),
  };
}

function devSeedOrderLines(index: number, isSalesReturn: boolean, padded: string) {
  const productCode = devSeedProductCode(index);
  const expectedQty = 20 + (index % 9) * 5;
  if (!isSalesReturn) {
    return [
      {
        line_no: 1,
        product_code: productCode,
        product_id: null,
        batch_no: null,
        expected_qty: expectedQty,
        production_date: "2026-01-01",
        expiry_date: "2028-01-01",
      },
    ];
  }

  const secondQty = Math.max(1, Math.floor(expectedQty / 3));
  return [
    {
      line_no: 1,
      product_code: productCode,
      product_id: null,
      batch_no: `SR-BATCH-${padded}-01`,
      expected_qty: expectedQty - secondQty,
      production_date: "2026-01-01",
      expiry_date: "2028-01-01",
    },
    {
      line_no: 2,
      product_code: productCode,
      product_id: null,
      batch_no: `SR-BATCH-${padded}-02`,
      expected_qty: secondQty,
      production_date: "2026-02-01",
      expiry_date: "2028-02-01",
    },
  ];
}

function allDevOrders() {
  return [...devSeedOrders(), ...devCreatedOrders];
}

function findDevOrder(id: string) {
  const seedOrderIndex = devSeedOrderIndex(id);
  if (seedOrderIndex !== null) return devSeedOrder(seedOrderIndex);
  return devCreatedOrders.find((order) => order.id === id) ?? null;
}

function setDevOrderStatus(id: string, status: string) {
  if (devSeedOrderIndex(id) !== null) {
    devSeedOrderStatusOverrides.set(id, status);
    return;
  }
  const order = devCreatedOrders.find((item) => item.id === id);
  if (!order) return;
  order.status = status;
  order.updated_at = new Date().toISOString();
}

function devOrderExpectedQty(id: string) {
  const order = findDevOrder(id);
  return order?.lines.reduce((total, line) => total + line.expected_qty, 0) ?? 0;
}

function devSeedOrderId(index: number) {
  return `00000000-0000-0000-0000-${String(2000 + index).padStart(12, "0")}`;
}

function devSeedOrderIndex(id: string) {
  const prefix = "00000000-0000-0000-0000-";
  if (!id.startsWith(prefix)) return null;
  const value = Number.parseInt(id.slice(prefix.length), 10) - 2000;
  if (!Number.isInteger(value) || value < 1 || value > devSeedOrderCount) return null;
  return value;
}

function devSeedDocumentType(index: number): DevOrder["document_type"] {
  return index % 5 === 0 ? "sales_return" : "purchase_inbound";
}

function devSeedOrderStatus(index: number) {
  return index % 2 === 0 ? "released" : "receiving";
}

function devSeedProductCode(index: number) {
  if (index % 6 === 0) return `P-M2-COLD-${String(index).padStart(3, "0")}`;
  return `P-M2-${String(index).padStart(3, "0")}`;
}

function devOrderFromCreateRequest(body: Record<string, unknown>): DevOrder {
  const now = new Date().toISOString();
  const lines = Array.isArray(body.lines) ? body.lines : [];
  const line = asRecord(lines[0]);
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    receipt_no: asString(body.receipt_no, `ASN-M2-PC-${Date.now()}`),
    document_type: asDocumentType(body.document_type),
    warehouse_id: asString(body.warehouse_id, devWarehouseId),
    status: "receiving",
    expected_arrival_at: asNullableString(body.expected_arrival_at),
    external_ref: asNullableString(body.external_ref),
    supplier_id: asNullableString(body.supplier_id),
    created_at: now,
    updated_at: now,
    lines: [
      {
        line_no: asNumber(line.line_no, 1),
        product_code: asString(line.product_code, "P-M2-NEW"),
        product_id: asNullableString(line.product_id),
        batch_no: asNullableString(line.batch_no),
        expected_qty: asNumber(line.expected_qty, 1),
        production_date: asNullableString(line.production_date),
        expiry_date: asNullableString(line.expiry_date),
      },
    ],
  };
}

async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
  let raw = "";
  for await (const chunk of req) {
    raw += String(chunk);
  }
  if (!raw) return {};
  const parsed: unknown = JSON.parse(raw);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(parsed)) {
    record[key] = value;
  }
  return record;
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    record[key] = item;
  }
  return record;
}

function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(body));
}

function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

function asNullableString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function asBoolean(value: unknown, fallback: boolean) {
  return typeof value === "boolean" ? value : fallback;
}

function asDocumentType(value: unknown): "purchase_inbound" | "sales_return" {
  if (value === "purchase_inbound" || value === "sales_return") return value;
  throw new Error("Invalid document_type");
}

export default defineConfig(({ command }) => {
  const devLoginEnabled = command === "serve" && process.env.WMS_WEB_ADMIN_DEV_LOGIN !== "0";

  return {
    define: {
      __WMS_WEB_ADMIN_DEV_LOGIN__: JSON.stringify(devLoginDefaults(devLoginEnabled)),
    },
    plugins: [react(), webAdminDevMock()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
    server: {
      host: "0.0.0.0",
      port: 9002,
      strictPort: true,
      proxy: e2eApiUrl
        ? {
            "/api": {
              target: e2eApiUrl,
              changeOrigin: true,
            },
          }
        : undefined,
    },
  };
});
