import type { IncomingMessage, ServerResponse } from "node:http";

import type {
  DevCustomer,
  DevFeatureFlagConfig,
  DevLocation,
  DevOrder,
  DevPrintTemplate,
  DevReceivingPrintData,
  DevSupplier,
  DevSystemDictionaryItem,
  DevWarehouse,
} from "./web-admin-dev-mock-model";
import * as model from "./web-admin-dev-mock-model";

const {
  devCreatedCustomers,
  devCreatedLocations,
  devCreatedOrders,
  devCreatedPrintTemplates,
  devCreatedSuppliers,
  devCreatedWarehouses,
  devFeatureFlags: initialDevFeatureFlags,
  devLocation,
  devLocationId,
  devOwnerId,
  devSeedOrderCount,
  devSeedOrderStatusOverrides,
  devSupplier,
  devSystemDictionaryItemsByCode,
  devUserId,
  devFeatureFlagSource: initialDevFeatureFlagSource,
  devWarehouse,
  devWarehouseId,
} = model;

const devFeatureFlags = [...initialDevFeatureFlags];
let devFeatureFlagSource = initialDevFeatureFlagSource;

export async function handleFeatureFlagRequest(req: IncomingMessage, res: ServerResponse, pathname: string) {
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
    devFeatureFlags.length = 0;
    devFeatureFlags.push(...flags);
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

  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Feature flag dev mock route not found");
}

export function devFeatureFlagConfig(body: Record<string, unknown>): DevFeatureFlagConfig {
  return {
    key: asString(body.key, "unknown.flag"),
    owner: asString(body.owner, "M1"),
    created_at: asString(body.created_at, new Date().toISOString().slice(0, 10)),
    cleanup_by: asString(body.cleanup_by, "2026-08-31"),
    enabled: asBoolean(body.enabled, false),
    source: asString(body.source, devFeatureFlagSource),
  };
}

export async function handleSystemDictionaryUpsert(
  req: IncomingMessage,
  res: ServerResponse,
  dictCode: string,
  itemCode: string,
) {
  const decodedDictCode = decodeURIComponent(dictCode);
  const decodedItemCode = decodeURIComponent(itemCode);
  const items = model.devSystemDictionaryItemsByCode[decodedDictCode];
  if (!items) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dictionary not found");
    return;
  }

  const body = await readJsonBody(req);
  const now = new Date().toISOString();
  const ownerId = asNullableString(body.owner_id);
  const existingIndex = items.findIndex((item) => item.item_code === decodedItemCode && item.owner_id === ownerId);
  const next: DevSystemDictionaryItem = {
    id: existingIndex >= 0 ? items[existingIndex].id : crypto.randomUUID(),
    dict_code: decodedDictCode,
    item_code: decodedItemCode,
    item_name: asString(body.item_name, decodedItemCode),
    owner_id: ownerId,
    params: asRecord(body.params),
    source: ownerId ? "owner" : "global",
    enabled: asBoolean(body.enabled, true),
    sort_order: asNumber(body.sort_order, existingIndex >= 0 ? items[existingIndex].sort_order : 0),
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

export async function handleSystemDictionaryDisable(
  req: IncomingMessage,
  res: ServerResponse,
  dictCode: string,
  itemCode: string,
) {
  const decodedDictCode = decodeURIComponent(dictCode);
  const decodedItemCode = decodeURIComponent(itemCode);
  const items = model.devSystemDictionaryItemsByCode[decodedDictCode];
  if (!items) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dictionary not found");
    return;
  }

  const body = await readJsonBody(req);
  const ownerId = asNullableString(body.owner_id);
  const index = items.findIndex((item) => item.item_code === decodedItemCode && item.owner_id === ownerId);
  if (index < 0) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dictionary item not found");
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

export async function handleSupplierUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedSuppliers.findIndex((supplier) => supplier.id === id);
  const supplier = id === model.devSupplier.id ? model.devSupplier : devCreatedSuppliers[createdIndex];
  if (!supplier) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Supplier not found");
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

  if (id === model.devSupplier.id) {
    Object.assign(model.devSupplier, updated);
  } else {
    devCreatedSuppliers[createdIndex] = updated;
  }
  sendJson(res, 200, updated);
}

export async function handleCustomerUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedCustomers.findIndex((customer) => customer.id === id);
  const customer = id === model.devCustomer.id ? model.devCustomer : devCreatedCustomers[createdIndex];
  if (!customer) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Customer not found");
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

  if (id === model.devCustomer.id) {
    Object.assign(model.devCustomer, updated);
  } else {
    devCreatedCustomers[createdIndex] = updated;
  }
  sendJson(res, 200, updated);
}

export async function handleWarehouseUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedWarehouses.findIndex((warehouse) => warehouse.id === id);
  const warehouse = id === model.devWarehouse.id ? model.devWarehouse : devCreatedWarehouses[createdIndex];
  if (!warehouse) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Warehouse not found");
    return;
  }

  const body = await readJsonBody(req);
  const updated: DevWarehouse = {
    ...warehouse,
    warehouse_name: asString(body.warehouse_name, warehouse.warehouse_name),
    warehouse_type: asString(body.warehouse_type, warehouse.warehouse_type),
    status: asString(body.status, warehouse.status),
    updated_at: new Date().toISOString(),
  };

  if (id === model.devWarehouse.id) {
    Object.assign(model.devWarehouse, updated);
  } else {
    devCreatedWarehouses[createdIndex] = updated;
  }
  sendJson(res, 200, updated);
}

export async function handleLocationUpdate(req: IncomingMessage, res: ServerResponse, id: string) {
  const createdIndex = devCreatedLocations.findIndex((location) => location.id === id);
  const location = id === model.devLocation.id ? model.devLocation : devCreatedLocations[createdIndex];
  if (!location) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Location not found");
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

  if (id === model.devLocation.id) {
    Object.assign(model.devLocation, updated);
  } else {
    devCreatedLocations[createdIndex] = updated;
  }
  sendJson(res, 200, updated);
}

export async function handleInboundAction(
  req: IncomingMessage,
  res: ServerResponse,
  action: string | undefined,
  orderId: string,
) {
  const body = await readJsonBody(req);
  const occurredAt = new Date().toISOString();

  const order = allDevOrders().find((item) => item.id === orderId);
  if (!order) {
    sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Receiving order not found");
    return;
  }

  if (action === "receive") {
    setDevOrderStatus(orderId, "inspecting");
    const receipt = {
      id: "00000000-0000-0000-0000-000000004001",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: asNumber(body.actual_qty, 120),
      shortage_qty: asNumber(body.shortage_qty, 0),
      rejected_qty: asNumber(body.rejected_qty, 0),
      arrival_temperature_celsius: typeof body.arrival_temperature_celsius === "number" ? body.arrival_temperature_celsius : null,
      exception_note: asNullableString(body.exception_note),
      details: receivingDetails(body.details),
      occurred_at: occurredAt,
    } satisfies DevReceivingPrintData["receipts"][number];
    const previous = model.devReceivingPrintData.get(orderId);
    model.devReceivingPrintData.set(orderId, {
      receipts: [receipt],
      inspections: previous?.inspections ?? [],
      signatures: previous?.signatures ?? [],
    });
    sendJson(res, 200, receipt);
    return;
  }

  if (action === "reject") {
    setDevOrderStatus(orderId, "closed_rejected");
    const receipt = {
      id: "00000000-0000-0000-0000-000000004005",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      actual_qty: 0,
      shortage_qty: 0,
      rejected_qty: devOrderExpectedQty(orderId),
      arrival_temperature_celsius: null,
      exception_note: asNullableString(body.reason),
      details: null,
      occurred_at: occurredAt,
    } satisfies DevReceivingPrintData["receipts"][number];
    const previous = model.devReceivingPrintData.get(orderId);
    model.devReceivingPrintData.set(orderId, {
      receipts: [receipt],
      inspections: previous?.inspections ?? [],
      signatures: previous?.signatures ?? [],
    });
    sendJson(res, 200, receipt);
    return;
  }

  if (action === "inspect") {
    const inspection = {
      id: "00000000-0000-0000-0000-000000004002",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      batch_no: asString(body.batch_no, "BATCH-202606"),
      accepted_qty: asNumber(body.accepted_qty, 120),
      rejected_qty: asNumber(body.rejected_qty, 0),
      quality_status: asString(body.quality_status, "qualified"),
      occurred_at: occurredAt,
    } satisfies DevReceivingPrintData["inspections"][number];
    const previous = model.devReceivingPrintData.get(orderId);
    model.devReceivingPrintData.set(orderId, {
      receipts: previous?.receipts ?? [],
      inspections: [...(previous?.inspections ?? []), inspection],
      signatures: previous?.signatures ?? [],
    });
    sendJson(res, 200, inspection);
    return;
  }

  if (action === "sign") {
    setDevOrderStatus(orderId, "putaway");
    const signature = {
      id: "00000000-0000-0000-0000-000000004003",
      receiving_order_id: orderId,
      owner_id: devOwnerId,
      first_signer_id: asString(body.first_signer_id, devUserId),
      second_signer_id: asNullableString(body.second_signer_id),
      signed_at: occurredAt,
    } satisfies DevReceivingPrintData["signatures"][number];
    const previous = model.devReceivingPrintData.get(orderId);
    model.devReceivingPrintData.set(orderId, {
      receipts: previous?.receipts ?? [],
      inspections: previous?.inspections ?? [],
      signatures: [...(previous?.signatures ?? []), signature],
    });
    sendJson(res, 200, signature);
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

  sendError(res, 404, "DEV_MOCK_NOT_FOUND", "Dev mock route not found");
}

export function allDevOrders(): DevOrder[] {
  return [...seedOrders(), ...devCreatedOrders];
}

export function getDevOrderPrintData(orderId: string) {
  const order = findOrder(orderId);
  if (!order) return null;
  const data = model.devReceivingPrintData.get(orderId);
  return {
    order,
    receipts: data?.receipts ?? [],
    inspections: data?.inspections ?? [],
    signatures: data?.signatures ?? [],
  };
}

function receivingDetails(value: unknown) {
  const details = asRecord(value);
  const salesReturnBatches = Array.isArray(details.sales_return_batches)
    ? details.sales_return_batches.map((item) => {
        const batch = asRecord(item);
        return {
          batch_no: asString(batch.batch_no, ""),
          quantity: asNumber(batch.quantity, 0),
          rejected_qty: asNumber(batch.rejected_qty, 0),
          reject_reason: asNullableString(batch.reject_reason),
        };
      })
    : [];
  return {
    delivery_qty: asNumber(details.delivery_qty, 0),
    temperature_control_method: asNullableString(details.temperature_control_method),
    vehicle_no: asNullableString(details.vehicle_no),
    origin: asNullableString(details.origin),
    departure_at: asNullableString(details.departure_at),
    arrival_at: asNullableString(details.arrival_at),
    storage_at: asNullableString(details.storage_at),
    transport_mode: asNullableString(details.transport_mode),
    carrier: asNullableString(details.carrier),
    contact_name: asNullableString(details.contact_name),
    contact_phone: asNullableString(details.contact_phone),
    contact_id_no: asNullableString(details.contact_id_no),
    seal_checked: asNullableString(details.seal_checked),
    filing_checked: asNullableString(details.filing_checked),
    second_receiver_id: asNullableString(details.second_receiver_id),
    sales_return_batches: salesReturnBatches,
  };
}

export function devOrderFromCreateRequest(body: Record<string, unknown>): DevOrder {
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

function seedOrders(): DevOrder[] {
  return Array.from({ length: devSeedOrderCount }, (_value, index) => makeSeedOrder(index + 1));
}

function makeSeedOrder(index: number): DevOrder {
  const now = new Date().toISOString();
  const id = seedOrderId(index);
  const documentType = seedDocumentType(index);
  const isSalesReturn = documentType === "sales_return";
  const padded = String(index).padStart(4, "0");
  return {
    id,
    owner_id: devOwnerId,
    receipt_no: `${isSalesReturn ? "SR" : "ASN"}-M2-PC-${padded}`,
    document_type: documentType,
    warehouse_id: devWarehouseId,
    status: devSeedOrderStatusOverrides.get(id) ?? seedOrderStatus(index),
    expected_arrival_at: new Date(Date.UTC(2026, 5, 27 + Math.floor((index - 1) / 24), index % 24)).toISOString(),
    external_ref: `${isSalesReturn ? "ERP-SR" : "ERP-ASN"}-${padded}`,
    supplier_id: `00000000-0000-0000-0000-${String(5000 + (index % 20)).padStart(12, "0")}`,
    created_at: "2026-06-27T08:00:00.000Z",
    updated_at: now,
    lines: seedOrderLines(index, isSalesReturn, padded),
  };
}

function seedOrderLines(index: number, isSalesReturn: boolean, padded: string): DevOrder["lines"] {
  const productCode = index % 6 === 0 ? `P-M2-COLD-${String(index).padStart(3, "0")}` : `P-M2-${String(index).padStart(3, "0")}`;
  const expectedQty = 20 + (index % 9) * 5;
  const line = (lineNo: number, batchNo: string | null, qty: number, month: string) => ({
    line_no: lineNo,
    product_code: productCode,
    product_id: null,
    batch_no: batchNo,
    expected_qty: qty,
    production_date: `2026-${month}-01`,
    expiry_date: `2028-${month}-01`,
  });
  if (!isSalesReturn) return [line(1, null, expectedQty, "01")];
  const secondQty = Math.max(1, Math.floor(expectedQty / 3));
  return [
    line(1, `SR-BATCH-${padded}-01`, expectedQty - secondQty, "01"),
    line(2, `SR-BATCH-${padded}-02`, secondQty, "02"),
  ];
}

export function devOrderExpectedQty(id: string): number {
  const order = allDevOrders().find((item) => item.id === id);
  return order?.lines.reduce((total, line) => total + line.expected_qty, 0) ?? 0;
}

export function setDevOrderStatus(id: string, status: string) {
  if (seedOrderIndex(id) !== null) {
    devSeedOrderStatusOverrides.set(id, status);
    return;
  }
  const order = devCreatedOrders.find((item) => item.id === id);
  if (!order) return;
  order.status = status;
  order.updated_at = new Date().toISOString();
}

function findOrder(id: string) {
  const index = seedOrderIndex(id);
  if (index !== null) return makeSeedOrder(index);
  return devCreatedOrders.find((item) => item.id === id) ?? null;
}

function seedOrderId(index: number) {
  return `00000000-0000-0000-0000-${String(2000 + index).padStart(12, "0")}`;
}

function seedOrderIndex(id: string) {
  const prefix = "00000000-0000-0000-0000-";
  if (!id.startsWith(prefix)) return null;
  const value = Number.parseInt(id.slice(prefix.length), 10) - 2000;
  return Number.isInteger(value) && value >= 1 && value <= devSeedOrderCount ? value : null;
}

function seedDocumentType(index: number): DevOrder["document_type"] {
  return index % 5 === 0 ? "sales_return" : "purchase_inbound";
}

function seedOrderStatus(index: number) {
  // 轮转状态，保证收货 / 验收 / 上架三页默认筛选下都有数据
  const statuses = ["released", "receiving", "inspecting", "putaway", "completed", "closed_rejected"] as const;
  return statuses[(index - 1) % statuses.length];
}

export async function readJsonBody(req: IncomingMessage): Promise<Record<string, unknown>> {
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

export function asRecord(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const record: Record<string, unknown> = {};
  for (const [key, item] of Object.entries(value)) {
    record[key] = item;
  }
  return record;
}

export function sendJson(res: ServerResponse, statusCode: number, body: unknown) {
  res.statusCode = statusCode;
  res.setHeader("content-type", "application/json; charset=utf-8");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(body));
}

export function sendError(
  res: ServerResponse,
  statusCode: number,
  code: string,
  message: string,
  details: Record<string, unknown> = {},
) {
  sendJson(res, statusCode, {
    code,
    message,
    severity: "error",
    details,
    trace_id: "dev-mock",
    retry_hint: null,
  });
}

export function asNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function asString(value: unknown, fallback: string) {
  return typeof value === "string" && value.trim() ? value.trim() : fallback;
}

export function asNullableString(value: unknown) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

export function asBoolean(value: unknown, fallback: boolean) {
  return typeof value === "boolean" ? value : fallback;
}

export function asDocumentType(value: unknown): "purchase_inbound" | "sales_return" {
  if (value === "purchase_inbound" || value === "sales_return") return value;
  throw new Error("Invalid document_type");
}
