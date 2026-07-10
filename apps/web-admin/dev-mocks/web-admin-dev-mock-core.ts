import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";

import { handleAdminMenuDevMock } from "./admin-menu-dev-mock";
import { handleH4WechatNotifyDevMock } from "./wechat-notify-dev-mock";
import { handleH5ExpressDevMock } from "./express-dev-mock";
import { handlePrintInventoryDevMock } from "./web-admin-dev-mock-print-inventory";

import {
  asNullableString,
  asNumber,
  asRecord,
  asString,
  allDevOrders,
  handleCustomerUpdate,
  handleFeatureFlagRequest,
  handleInboundAction,
  handleLocationUpdate,
  handleProductUpdate,
  handleSystemDictionaryDisable,
  handleSystemDictionaryUpsert,
  devOrderExpectedQty,
  devOrderFromCreateRequest,
  handleSupplierUpdate,
  handleWarehouseUpdate,
  readJsonBody,
  sendError,
  sendJson,
} from "./web-admin-dev-mock-core-common";
import { devLoginDefaults } from "./web-admin-dev-mock-model";
import * as model from "./web-admin-dev-mock-model";
import type {
  DevCustomer,
  DevLocation,
  DevProduct,
  DevSupplier,
  DevWarehouse,
} from "./web-admin-dev-mock-model";

const {
  devCreatedCustomers,
  devCreatedLocations,
  devCreatedOrders,
  devCreatedProducts,
  devCreatedSuppliers,
  devCreatedWarehouses,
  devLocation,
  devLocationId,
  devLoginPassword,
  devMockEnabled,
  devOwnerId,
  devCustomer,
  devSeedProducts,
  devSupplier,
  devSystemDictionaryItemsByCode,
  devUser,
  devWarehouse,
  devWarehouseId,
} = model;

interface DevLocationBatchIdempotencyEntry {
  requestBody: string;
  responseBody: { data: DevLocation[]; page: { count: number; next_cursor: null } };
}

const devLocationBatchIdempotency = new Map<string, DevLocationBatchIdempotencyEntry>();

export { devLoginDefaults };

export function webAdminDevMock(): Plugin {
  return {
    name: "wms-web-admin-dev-mock",
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (!devMockEnabled || !req.url || !req.url.startsWith("/api/")) {
          next();
          return;
        }

        const pathname = new URL(req.url, "http://wms.local").pathname;

        try {
          if (await tryHandleDevMockRoute(req, res, pathname)) return;
          sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dev mock route not found");
        } catch (error) {
          if (error instanceof SyntaxError) {
            sendError(res, 400, "DEV_MOCK_REQUEST_INVALID", "请求体不是有效 JSON");
            return;
          }
          sendError(res, 500, "DEV_MOCK_ERROR", error instanceof Error ? error.message : "Dev mock failed");
        }
      });
    },
  };
}

async function tryHandleDevMockRoute(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (pathname.startsWith("/api/v1/admin/menus")) {
    await handleAdminMenuDevMock(req, res, pathname);
    return true;
  }
  if (pathname.startsWith("/api/v1/express")) {
    await handleH5ExpressDevMock(req, res, pathname);
    return true;
  }
  if (pathname.startsWith("/api/v1/wechat-notify")) {
    await handleH4WechatNotifyDevMock(req, res, pathname);
    return true;
  }

  if (req.method === "POST" && pathname === "/api/v1/auth/login") {
    const body = await readJsonBody(req);
    const valid =
      body.owner_code === "PY_OWNER" &&
      body.username === "admin" &&
      body.password === devLoginPassword;
    if (!valid) {
      sendError(res, 401, "AUTH_INVALID_CREDENTIALS", "Login failed");
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

  if (pathname === "/api/v1/auth/me") {
    if (req.method !== "GET") {
      sendError(res, 405, "METHOD_NOT_ALLOWED", "Not allowed");
      return true;
    }
    sendJson(res, 200, devUser);
    return true;
  }

  if (req.method === "GET") {
    const response = devMasterDataResponse(pathname);
    if (response) {
      sendJson(res, 200, response);
      return true;
    }
  }

  if (pathname === "/api/v1/master-data/products" && req.method === "POST") {
    const body = await readJsonBody(req);
    const next = devCreateProduct(body);
    devCreatedProducts.unshift(next);
    sendJson(res, 200, next);
    return true;
  }

  const productUpdate = matchUpdate(pathname, "/api/v1/master-data/products/");
  if (productUpdate && req.method === "PATCH") {
    await handleProductUpdate(req, res, productUpdate);
    return true;
  }

  if (pathname === "/api/v1/master-data/suppliers" && req.method === "POST") {
    const body = await readJsonBody(req);
    const next = devSupplierFromCreateRequest(body);
    devCreatedSuppliers.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  const supplierId = matchUpdate(pathname, "/api/v1/master-data/suppliers/");
  if (supplierId && req.method === "PATCH") {
    await handleSupplierUpdate(req, res, supplierId);
    return true;
  }

  if (pathname === "/api/v1/master-data/customers" && req.method === "POST") {
    const body = await readJsonBody(req);
    const next = devCustomerFromCreateRequest(body);
    devCreatedCustomers.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  const customerId = matchUpdate(pathname, "/api/v1/master-data/customers/");
  if (customerId && req.method === "PATCH") {
    await handleCustomerUpdate(req, res, customerId);
    return true;
  }

  if (pathname === "/api/v1/master-data/warehouses" && req.method === "POST") {
    const body = await readJsonBody(req);
    const next = devWarehouseFromCreateRequest(body);
    devCreatedWarehouses.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  const warehouseId = matchUpdate(pathname, "/api/v1/master-data/warehouses/");
  if (warehouseId && req.method === "PATCH") {
    await handleWarehouseUpdate(req, res, warehouseId);
    return true;
  }

  if (pathname === "/api/v1/master-data/locations" && req.method === "POST") {
    const body = await readJsonBody(req);
    const next = devLocationFromCreateRequest(body);
    if (!next) {
      sendError(res, 422, "DEV_MOCK_REQUEST_INVALID", "库位创建参数非法");
      return true;
    }
    devCreatedLocations.unshift(next);
    sendJson(res, 200, next);
    return true;
  }
  if (pathname === "/api/v1/master-data/locations/batch-create" && req.method === "POST") {
    const idempotencyKey = getIdempotencyKey(req);
    if (!idempotencyKey) {
      sendError(res, 400, "M1_LOCATION_IDEMPOTENCY_REQUIRED", "缺少 Idempotency-Key");
      return true;
    }
    const body = await readJsonBody(req);
    const requestBody = JSON.stringify(body);
    const replay = devLocationBatchIdempotency.get(idempotencyKey);
    if (replay) {
      if (replay.requestBody !== requestBody) {
        sendError(res, 409, "M1_LOCATION_IDEMPOTENCY_CONFLICT", "Idempotency-Key 已用于其他请求");
      } else {
        sendJson(res, 200, replay.responseBody);
      }
      return true;
    }
    const created = devLocationsFromBatchRequest(body);
    if (!created) {
      sendError(res, 422, "M1_LOCATION_BATCH_INVALID", "库位批量创建范围非法");
      return true;
    }
    const existingCodes = new Set([devLocation, ...devCreatedLocations].map((item) => item.location_code));
    if (created.some((item) => existingCodes.has(item.location_code))) {
      sendError(res, 409, "M1_LOCATION_DUPLICATE", "库位编码已存在");
      return true;
    }
    devCreatedLocations.unshift(...created);
    const responseBody = { data: created, page: { count: created.length, next_cursor: null } };
    devLocationBatchIdempotency.set(idempotencyKey, { requestBody, responseBody });
    sendJson(res, 200, responseBody);
    return true;
  }
  const locationId = matchUpdate(pathname, "/api/v1/master-data/locations/");
  if (locationId && req.method === "PATCH") {
    await handleLocationUpdate(req, res, locationId);
    return true;
  }

  const dictItem = pathname.match(/^\/api\/v1\/system-dictionaries\/([^/]+)\/items\/([^/]+)$/);
  if (dictItem && req.method === "PUT") {
    await handleSystemDictionaryUpsert(req, res, dictItem[1], dictItem[2]);
    return true;
  }

  if (dictItem && req.method === "PATCH") {
    await handleSystemDictionaryDisable(req, res, dictItem[1], dictItem[2]);
    return true;
  }

  const dictItemsPostfix = pathname.match(/^\/api\/v1\/system-dictionaries\/([^/]+)\/items\/([^/]+)\/disable$/);
  if (dictItemsPostfix && req.method === "PATCH") {
    await handleSystemDictionaryDisable(req, res, dictItemsPostfix[1], dictItemsPostfix[2]);
    return true;
  }

  if (pathname.startsWith("/api/v1/config-center/feature-flags")) {
    await handleFeatureFlagRequest(req, res, pathname);
    return true;
  }

  if (await handlePrintInventoryDevMock(req, res, pathname)) return true;

  if (req.method === "GET" && pathname === "/api/v1/inbound/receiving-orders") {
    const data = allDevOrders();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }

  const receivingOrderIdMatch = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)$/);
  if (receivingOrderIdMatch && req.method === "GET") {
    const id = decodeURIComponent(receivingOrderIdMatch[1]);
    const order = allDevOrders().find((item) => item.id === id);
    if (!order) {
      sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Receiving order not found");
      return true;
    }
    sendJson(res, 200, order);
    return true;
  }

  const receivingAction = pathname.match(/^\/api\/v1\/inbound\/receiving-orders\/([^/]+)\/(receive|reject|inspect|sign|putaway)$/);
  if (receivingAction && req.method === "POST") {
    const [, orderId, action] = receivingAction;
    await handleInboundAction(req, res, action, orderId);
    return true;
  }

  if (pathname === "/api/v1/inbound/receiving-orders" && req.method === "POST") {
    const body = await readJsonBody(req);
    let next: ReturnType<typeof devOrderFromCreateRequest>;
    try {
      next = devOrderFromCreateRequest(body);
    } catch {
      sendError(res, 422, "W3-422", "document_type 必须是 purchase_inbound 或 sales_return");
      return true;
    }
    devCreatedOrders.unshift(next);
    sendJson(res, 200, next);
    return true;
  }

  return false;
}

function devMasterDataResponse(pathname: string): Record<string, unknown> | null {
  let data: unknown[] | undefined;
  if (pathname === "/api/v1/master-data/products") {
    data = [...devCreatedProducts, ...devSeedProducts];
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
  if (pathname === "/api/v1/master-data/suppliers") data = [...devCreatedSuppliers, devSupplier];
  if (pathname === "/api/v1/master-data/customers") data = [...devCreatedCustomers, devCustomer];
  if (pathname === "/api/v1/master-data/warehouses") data = [...devCreatedWarehouses, devWarehouse];
  if (pathname === "/api/v1/master-data/locations") data = [...devCreatedLocations, devLocation];
  const dictionary = pathname.match(/^\/api\/v1\/system-dictionaries\/([^/]+)\/items$/);
  if (dictionary) data = devSystemDictionaryItemsByCode[decodeURIComponent(dictionary[1])];
  return data ? { data, page: { count: data.length, next_cursor: null } } : null;
}

function matchUpdate(pathname: string, prefix: string) {
  if (!pathname.startsWith(prefix)) return null;
  const id = decodeURIComponent(pathname.slice(prefix.length));
  return id && !id.includes("/") ? id : null;
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

function devCreateProduct(body: Record<string, unknown>): DevProduct {
  return devProductFromCreateRequest(body);
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

function devLocationFromCreateRequest(body: Record<string, unknown>): DevLocation | null {
  if (!isValidLocationInput(body)) return null;
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

function devLocationsFromBatchRequest(body: Record<string, unknown>): DevLocation[] | null {
  const areaCode = asString(body.area_code, "").toUpperCase();
  const rowStart = asNumber(body.row_start, 0);
  const rowEnd = asNumber(body.row_end, 0);
  const columnStart = asNumber(body.column_start, 0);
  const columnEnd = asNumber(body.column_end, 0);
  const layerStart = asNumber(body.layer_start, 0);
  const layerEnd = asNumber(body.layer_end, 0);
  const total = (rowEnd - rowStart + 1) * (columnEnd - columnStart + 1) * (layerEnd - layerStart + 1);
  const validSharedInput = isValidLocationInput({
    ...body,
    location_code: `${areaCode}-01-01-01`,
    row_no: rowStart,
    column_no: columnStart,
    layer_no: layerStart,
  });
  if (
    !/^[A-Z0-9]{3}$/.test(areaCode)
    || rowStart < 1 || rowStart > rowEnd || rowEnd > 99
    || columnStart < 1 || columnStart > columnEnd || columnEnd > 99
    || layerStart < 1 || layerStart > layerEnd || layerEnd > 99
    || ![rowStart, rowEnd, columnStart, columnEnd, layerStart, layerEnd].every(Number.isSafeInteger)
    || total > 500
    || !validSharedInput
  ) return null;

  const locations: DevLocation[] = [];
  for (let row = rowStart; row <= rowEnd; row += 1) {
    for (let column = columnStart; column <= columnEnd; column += 1) {
      for (let layer = layerStart; layer <= layerEnd; layer += 1) {
        const location = devLocationFromCreateRequest({
          ...body,
          location_code: `${areaCode}-${pad2(row)}-${pad2(column)}-${pad2(layer)}`,
          row_no: row,
          column_no: column,
          layer_no: layer,
        });
        if (!location) return null;
        locations.push(location);
      }
    }
  }
  return locations;
}

function isValidLocationInput(body: Record<string, unknown>) {
  const warehouseId = asString(body.warehouse_id, "");
  const zoneId = asString(body.zone_id, "");
  const locationCode = asString(body.location_code, "");
  const rowNo = asNumber(body.row_no, Number.NaN);
  const columnNo = asNumber(body.column_no, Number.NaN);
  const layerNo = asNumber(body.layer_no, Number.NaN);
  const maxVolumeCm3 = asNumber(body.max_volume_cm3, Number.NaN);
  const maxSkuCount = asNumber(body.max_sku_count, Number.NaN);
  const locationType = asString(body.location_type, "");
  const boundOwnerId = body.bound_owner_id === null || body.bound_owner_id === undefined
    ? null
    : asString(body.bound_owner_id, "");
  const validLocationType = (devSystemDictionaryItemsByCode.location_type ?? [])
    .some((item) => item.enabled && item.item_code === locationType);
  return warehouseId === devWarehouseId
    && isUuid(warehouseId)
    && zoneId === devLocation.zone_id
    && isUuid(zoneId)
    && locationCode.length > 0
    && [rowNo, columnNo, layerNo, maxVolumeCm3, maxSkuCount].every(Number.isSafeInteger)
    && [rowNo, columnNo, layerNo].every((value) => value >= 1 && value <= 99)
    && maxVolumeCm3 >= 0
    && maxSkuCount >= 1
    && maxSkuCount <= 2_147_483_647
    && validLocationType
    && (boundOwnerId === null || (boundOwnerId === devOwnerId && isUuid(boundOwnerId)));
}

function pad2(value: number) {
  return String(value).padStart(2, "0");
}

function getIdempotencyKey(req: IncomingMessage) {
  const key = req.headers["idempotency-key"];
  return Array.isArray(key) ? key[0]?.trim() || null : key?.trim() || null;
}

function isUuid(value: string) {
  return /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(value);
}
