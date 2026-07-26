import type { IncomingMessage, ServerResponse } from "node:http";

import {
  asBoolean,
  asNullableString,
  asRecord,
  asString,
  readJsonBody,
  sendJson,
} from "./web-admin-dev-mock-core-common";
import {
  devCreatedPrintTemplates,
  devLocation,
  devLocationId,
  devOwnerId,
  devPrintTemplateVersions,
  devSystemDictionaryItemsByCode,
  devUserId,
  type DevInventoryBatch,
  type DevPrintTemplate,
} from "./web-admin-dev-mock-model";

const inventoryBatchOverrides = new Map<string, Partial<DevInventoryBatch>>();
const inventoryRecallPreviousStatus = new Map<string, string>();
interface DevFieldLibrary {
  id: string;
  library_code: string;
  library_name: string;
  business_module: string;
  source_schema: string;
  latest_version_id: string;
  version_no: number;
  latest_version_status: string;
  latest_published_version_id: string | null;
  latest_published_version_no: number | null;
  field_count: number;
  created_at: string;
  created_by: string;
  published_at: string | null;
  published_by: string | null;
}
let devDraftFieldLibrary: DevFieldLibrary | null = null;
const devFieldDefinitionOverrides = new Map<string, Record<string, unknown>>();

export async function handlePrintInventoryDevMock(
  req: IncomingMessage,
  res: ServerResponse,
  pathname: string,
): Promise<boolean> {
  if (req.method === "GET" && pathname === "/api/v1/inventory/batches") {
    const data = inventoryBatches();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }
  if (req.method === "GET" && pathname === "/api/v1/inventory/locations/history") {
    const url = new URL(req.url ?? "/", "http://localhost");
    const locationCode = url.searchParams.get("location_code")?.trim() ?? "";
    if (!locationCode) {
      sendJson(res, 422, { code: "M3_LOCATION_REQUIRED", message: "库位编码不能为空" });
      return true;
    }
    const data = inventoryBatches()
      .filter((batch) => batch.location_code.includes(locationCode))
      .map((batch) => ({
        id: `00000000-0000-0000-0000-0000000070${batch.id.slice(-2)}`,
        owner_id: batch.owner_id,
        batch_id: batch.id,
        movement_type: "inbound_putaway",
        qty_delta: batch.qty_on_hand,
        source_document_type: "receiving_order",
        source_document_id: "00000000-0000-0000-0000-000000001901",
        occurred_at: batch.created_at,
        location_code: batch.location_code,
        from_location_code: null,
        to_location_code: batch.location_code,
        lpn_code: null,
        operator_user_id: devUserId,
        operator_name: "dev-keeper",
        volume_delta_cm3: null,
        product_code: batch.product_code,
        product_name: null,
        batch_no: batch.batch_no,
        expiry_date: batch.expiry_date,
      }));
    sendJson(res, 200, {
      location_code: locationCode,
      data,
      risks: data.some((item) => item.product_code.includes("COLD"))
        ? [{
          risk_code: "temperature_mismatch",
          severity: "high",
          message: `库位 ${locationCode} 历史存在冷链相关商品记录，需复核清洁状态`,
        }]
        : [],
      product_shares: Object.values(
        data.reduce<Record<string, { product_code: string; product_name: string | null; event_count: number; total_qty_delta: number }>>((summary, item) => {
          const key = item.product_code;
          const current = summary[key] ?? {
            product_code: key,
            product_name: item.product_name,
            event_count: 0,
            total_qty_delta: 0,
          };
          current.event_count += 1;
          current.total_qty_delta += item.qty_delta;
          summary[key] = current;
          return summary;
        }, {}),
      ),
      page: { count: data.length, next_cursor: null },
    });
    return true;
  }
  const trace = pathname.match(/^\/api\/v1\/inventory\/batches\/([^/]+)\/trace$/);
  if (req.method === "GET" && trace) {
    const batch = inventoryBatches().find((item) => item.id === decodeURIComponent(trace[1]));
    if (!batch) {
      sendJson(res, 404, { code: "M3_BATCH_NOT_FOUND", message: "库存批次不存在" });
      return true;
    }
    sendJson(res, 200, {
      batch,
      movements: [{
        id: `00000000-0000-0000-0000-0000000069${batch.id.slice(-2)}`,
        owner_id: batch.owner_id,
        batch_id: batch.id,
        movement_type: "inbound_putaway",
        qty_delta: batch.qty_on_hand,
        source_document_type: "receiving_order",
        source_document_id: "00000000-0000-0000-0000-000000001901",
        occurred_at: batch.created_at,
      }],
      status_changes: batch.recall_flag ? [{
        id: "00000000-0000-0000-0000-000000001902",
        owner_id: batch.owner_id,
        batch_id: batch.id,
        from_status: "qualified",
        to_status: "quarantined",
        reason: "召回标记",
        approval_source: "M-QL",
        approval_id: "QL-DEV-001",
        occurred_at: batch.updated_at,
      }] : [],
    });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/inventory/batches/status") {
    const body = await readJsonBody(req);
    const batchId = asString(body.batch_id, "");
    const targetStatus = asString(body.target_status, "");
    const batch = inventoryBatches().find((item) => item.id === batchId);
    if (!batch) {
      sendJson(res, 404, { code: "M3_BATCH_NOT_FOUND", message: "库存批次不存在" });
      return true;
    }
    inventoryBatchOverrides.set(batchId, { quality_status: targetStatus, updated_at: new Date().toISOString() });
    sendJson(res, 200, { ...batch, quality_status: targetStatus, updated_at: new Date().toISOString() });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/inventory/batches/recall") {
    const body = await readJsonBody(req);
    const batchId = asString(body.batch_id, "");
    const batch = inventoryBatches().find((item) => item.id === batchId);
    if (!batch) {
      sendJson(res, 404, { code: "M3_BATCH_NOT_FOUND", message: "库存批次不存在" });
      return true;
    }
    inventoryRecallPreviousStatus.set(batchId, batch.quality_status);
    const updatedAt = new Date().toISOString();
    inventoryBatchOverrides.set(batchId, { quality_status: "quarantined", recall_flag: true, updated_at: updatedAt });
    sendJson(res, 200, { ...batch, quality_status: "quarantined", recall_flag: true, updated_at: updatedAt });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/inventory/batches/recall/cancel") {
    const body = await readJsonBody(req);
    const batchId = asString(body.batch_id, "");
    const secondApproverId = asString(body.second_approver_id, "");
    const batch = inventoryBatches().find((item) => item.id === batchId);
    if (!batch) {
      sendJson(res, 404, { code: "M3_BATCH_NOT_FOUND", message: "库存批次不存在" });
      return true;
    }
    if (!batch.recall_flag) {
      sendJson(res, 409, { code: "M3_RECALL_NOT_ACTIVE", message: "该批次当前没有有效召回标记" });
      return true;
    }
    if (!secondApproverId) {
      sendJson(res, 422, { code: "M3_SECOND_APPROVER_REQUIRED", message: "必须提供质量审批人 ID" });
      return true;
    }
    const updatedAt = new Date().toISOString();
    const qualityStatus = inventoryRecallPreviousStatus.get(batchId) ?? batch.quality_status;
    inventoryBatchOverrides.set(batchId, { quality_status: qualityStatus, recall_flag: false, updated_at: updatedAt });
    inventoryRecallPreviousStatus.delete(batchId);
    sendJson(res, 200, { ...batch, quality_status: qualityStatus, recall_flag: false, updated_at: updatedAt });
    return true;
  }
  if (req.method === "GET" && pathname === "/api/v1/print-templates/field-libraries") {
    const data = fieldLibraries();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/print-templates/field-libraries/drafts") {
    const body = await readJsonBody(req);
    const now = new Date().toISOString();
    const versionId = crypto.randomUUID();
    devDraftFieldLibrary = {
      id: crypto.randomUUID(),
      library_code: asString(body.library_code, "h9_draft"),
      library_name: asString(body.library_name, "H9 字段库草稿"),
      business_module: asString(body.business_module, "H9"),
      source_schema: asString(body.source_schema, "CreateReceivingOrderRequest"),
      latest_version_id: versionId,
      version_no: 1,
      latest_version_status: "draft",
      latest_published_version_id: null,
      latest_published_version_no: null,
      field_count: 2,
      created_at: now,
      created_by: devUserId,
      published_at: null,
      published_by: null,
    };
    sendJson(res, 200, fieldLibraryVersion(devDraftFieldLibrary));
    return true;
  }
  const fields = pathname.match(
    /^\/api\/v1\/print-templates\/field-libraries\/([^/]+)\/fields$/,
  );
  if (req.method === "GET" && fields) {
    const data = fieldDefinitions(decodeURIComponent(fields[1]));
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }
  const fieldUpdate = pathname.match(
    /^\/api\/v1\/print-templates\/field-libraries\/([^/]+)\/fields\/([^/]+)$/,
  );
  if (req.method === "PATCH" && fieldUpdate) {
    const versionId = decodeURIComponent(fieldUpdate[1]);
    const fieldId = decodeURIComponent(fieldUpdate[2]);
    const current = fieldDefinitions(versionId).find((field) => field.id === fieldId);
    if (!current) {
      sendJson(res, 404, { code: "H9_FIELD_LIBRARY_VERSION_NOT_FOUND", message: "字段不存在" });
      return true;
    }
    const updated = { ...current, ...(await readJsonBody(req)) };
    devFieldDefinitionOverrides.set(fieldId, updated);
    sendJson(res, 200, updated);
    return true;
  }
  const publishLibrary = pathname.match(
    /^\/api\/v1\/print-templates\/field-libraries\/([^/]+)\/publish$/,
  );
  if (req.method === "POST" && publishLibrary && devDraftFieldLibrary?.latest_version_id === decodeURIComponent(publishLibrary[1])) {
    const now = new Date().toISOString();
    devDraftFieldLibrary = {
      ...devDraftFieldLibrary,
      latest_version_status: "published",
      latest_published_version_id: devDraftFieldLibrary.latest_version_id,
      latest_published_version_no: devDraftFieldLibrary.version_no,
      published_at: now,
      published_by: devUserId,
    };
    sendJson(res, 200, fieldLibraryVersion(devDraftFieldLibrary));
    return true;
  }
  if (req.method === "GET" && pathname === "/api/v1/print-templates/templates") {
    const data = templates();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }
  const versions = pathname.match(
    /^\/api\/v1\/print-templates\/templates\/([^/]+)\/versions$/,
  );
  if (req.method === "GET" && versions) {
    const data = templateVersions(decodeURIComponent(versions[1]));
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/print-templates/templates") {
    sendJson(res, 200, await saveTemplate(req));
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/print-templates/resolve") {
    const body = await readJsonBody(req);
    const template = matchingTemplate(body);
    sendJson(res, 200, { template, version: templateVersion(template) });
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/print-templates/preview") {
    sendJson(res, 200, preview(await readJsonBody(req)));
    return true;
  }
  if (req.method === "POST" && pathname === "/api/v1/print-templates/print") {
    sendJson(res, 200, printRecord(await readJsonBody(req)));
    return true;
  }
  return false;
}

function inventoryBatches(): DevInventoryBatch[] {
  const now = "2026-06-29T00:00:00.000Z";
  const rows = [
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
  return rows.map((row) => ({ ...row, ...inventoryBatchOverrides.get(row.id) }));
}

function seedFieldLibraries(): DevFieldLibrary[] {
  return (devSystemDictionaryItemsByCode.print_template_type ?? []).map((type, index) => {
    const libraryCode = String(type.params.field_library_code ?? type.item_code);
    return {
      id: `00000000-0000-0000-0000-0000000028${String(index + 1).padStart(2, "0")}`,
      library_code: libraryCode,
      library_name: `${type.item_name}字段库`,
      business_module: String(type.params.business_module ?? "H9"),
      source_schema: sourceSchema(libraryCode),
      latest_version_id: `00000000-0000-0000-0000-0000000029${String(index + 1).padStart(2, "0")}`,
      version_no: 1,
      latest_version_status: "published",
      latest_published_version_id: `00000000-0000-0000-0000-0000000029${String(index + 1).padStart(2, "0")}`,
      latest_published_version_no: 1,
      field_count: fieldCount(libraryCode),
      created_at: type.created_at,
      created_by: devUserId,
      published_at: type.updated_at,
      published_by: devUserId,
    };
  });
}

function fieldLibraries() {
  const rows = seedFieldLibraries();
  if (devDraftFieldLibrary) {
    const index = rows.findIndex((item) => item.library_code === devDraftFieldLibrary?.library_code);
    if (index >= 0) rows[index] = devDraftFieldLibrary;
    else rows.unshift(devDraftFieldLibrary);
  }
  return rows;
}

function fieldDefinitions(versionId: string) {
  const library = fieldLibraries().find(
    (item) => item.latest_version_id === versionId || item.latest_published_version_id === versionId,
  );
  const libraryCode = library?.library_code ?? "m2_asn";
  const paths = library === devDraftFieldLibrary
    ? [
        field("receipt_no", "receipt_no", "base", "base", "ASN-DEV-001"),
        field("lines[].product_code", "product_code", "lines", "lines", "P-DEV-001"),
      ]
    : fieldPaths(libraryCode);
  return paths.map((field, index) => {
    const id = `00000000-0000-0000-0000-000000004${String(index + 1).padStart(3, "0")}`;
    return {
    id,
    library_version_id: versionId,
    field_path: field.fieldPath,
    field_type: "string",
    source_schema: sourceSchema(libraryCode),
    display_name: field.displayName,
    group_code: field.groupCode,
    group_name: field.groupName,
    description: "",
    example_value: field.sampleValue,
    printable: true,
    sensitive: false,
    masking_rule: null,
    formatting_rule: null,
    supports_barcode: false,
    supports_qrcode: false,
    is_table_detail: field.fieldPath.includes("[]"),
    sort_order: (index + 1) * 10,
    ...devFieldDefinitionOverrides.get(id),
  };
  });
}

function fieldLibraryVersion(library: DevFieldLibrary) {
  return {
    id: library.latest_version_id,
    library_id: library.id,
    library_code: library.library_code,
    library_name: library.library_name,
    business_module: library.business_module,
    source_schema: library.source_schema,
    version_no: library.version_no,
    status: library.latest_version_status,
    created_at: library.created_at,
    created_by: library.created_by,
    published_at: library.published_at,
    published_by: library.published_by,
  };
}

function templates() {
  const rows = [...devCreatedPrintTemplates];
  const seed = seedTemplate();
  if (!rows.some((row) => row.id === seed.id)) rows.push(seed);
  return rows;
}

function seedTemplate(): DevPrintTemplate {
  const now = "2026-07-07T09:00:00.000Z";
  return {
    id: "00000000-0000-0000-0000-000000003801",
    template_code: "asn_default",
    template_name: "ASN 单默认模板",
    template_type_code: "asn",
    owner_id: devOwnerId,
    scope: "global",
    enabled: true,
    is_default: true,
    remark: "dev mock hiprint 模板",
    latest_version_id: "00000000-0000-0000-0000-000000003901",
    latest_version_no: 1,
    latest_version_status: "published",
    field_library_version_id: fieldLibraries()[0].latest_version_id,
    designer_version: "hiprint@0.4.0",
    created_at: now,
    updated_at: now,
    published_at: now,
    hiprint_json: hiprintJson("asn.code"),
    field_bindings: [{ field_path: "asn.code", required: true }],
    paper: { paperType: "A4", width: 210, height: 297, direction: "portrait" },
  };
}

async function saveTemplate(req: IncomingMessage) {
  const body = await readJsonBody(req);
  const now = new Date().toISOString();
  const code = asString(body.template_code, "h9_template");
  const typeCode = asString(body.template_type_code, "asn");
  const existing = templates().find(
    (item) => item.template_code === code && item.template_type_code === typeCode,
  );
  const previous = existing ? templateVersions(existing.id) : [];
  const template: DevPrintTemplate = {
    id: existing?.id ?? crypto.randomUUID(),
    template_code: code,
    template_name: asString(body.template_name, "H9 打印模板"),
    template_type_code: typeCode,
    owner_id: devOwnerId,
    scope: asString(body.scope, "global") === "owner" ? "owner" : "global",
    enabled: asBoolean(body.enabled, true),
    is_default: asBoolean(body.is_default, true),
    remark: asNullableString(body.remark),
    latest_version_id: crypto.randomUUID(),
    latest_version_no: (existing?.latest_version_no ?? 0) + 1,
    latest_version_status: asBoolean(body.publish, true) ? "published" : "draft",
    field_library_version_id: asString(
      body.field_library_version_id,
      fieldLibraries()[0].latest_version_id,
    ),
    designer_version: asString(body.designer_version, "hiprint@0.4.0"),
    created_at: existing?.created_at ?? now,
    updated_at: now,
    published_at: asBoolean(body.publish, true) ? now : null,
    hiprint_json: asRecord(body.hiprint_json),
    field_bindings: bindings(body.field_bindings),
    paper: asRecord(body.paper),
  };
  const index = devCreatedPrintTemplates.findIndex((item) => item.id === template.id);
  if (index >= 0) devCreatedPrintTemplates[index] = template;
  else devCreatedPrintTemplates.unshift(template);
  devPrintTemplateVersions.set(template.id, [template, ...previous]);
  return templateVersion(template);
}

function templateVersions(templateId: string) {
  const versions = devPrintTemplateVersions.get(templateId);
  if (versions) return versions.map(templateVersion);
  const template = templates().find((item) => item.id === templateId);
  return template ? [templateVersion(template)] : [];
}

function matchingTemplate(body: Record<string, unknown>) {
  const code = asNullableString(body.template_code);
  const typeCode = asString(body.template_type_code, "asn");
  return (
    templates().find(
      (item) => item.template_code === code && item.template_type_code === typeCode,
    ) ??
    templates().find((item) => item.template_type_code === typeCode) ??
    templates()[0]
  );
}

function preview(body: Record<string, unknown>) {
  const template = matchingTemplate(body);
  return {
    template_id: template.id,
    template_version_id: template.latest_version_id,
    template_code: template.template_code,
    template_name: template.template_name,
    template_type_code: template.template_type_code,
    version_no: template.latest_version_no,
    hiprint_json: template.hiprint_json,
    field_bindings: template.field_bindings,
    paper: template.paper,
    data: asRecord(body.data),
  };
}

function printRecord(body: Record<string, unknown>) {
  const value = preview(body);
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    owner_id: devOwnerId,
    template_version_id: value.template_version_id,
    business_module: asString(body.business_module, "H9"),
    business_document_type: asString(body.business_document_type, value.template_type_code),
    business_document_id: asString(body.business_document_id, "H9-SAMPLE"),
    status: asString(body.status, "printed"),
    failure_reason: asNullableString(body.failure_reason),
    retry_count: 0,
    printed_at: now,
    operator_id: devUserId,
    created_at: now,
  };
}

function templateVersion(template: DevPrintTemplate) {
  return {
    ...template,
    template_id: template.id,
    version_no: template.latest_version_no,
    status: template.latest_version_status,
    created_by: devUserId,
    published_by: template.published_at ? devUserId : null,
  };
}

function bindings(value: unknown) {
  return Array.isArray(value)
    ? value.map((item) => ({
        field_path: asString(asRecord(item).field_path, "asn.code"),
        required: asBoolean(asRecord(item).required, false),
      }))
    : [{ field_path: "asn.code", required: true }];
}

function fieldPaths(libraryCode: string) {
  if (libraryCode === "m2_asn") {
    return [
      field("asn.code", "ASN 号", "order", "订单信息", "ASN-202607070001"),
      field("supplier.name", "供应商", "order", "订单信息", "华东医药供应链"),
      field("product.name", "商品名称", "product", "商品信息", "冷藏胰岛素注射液"),
      field("product.spec", "规格", "product", "商品信息", "3ml:300单位"),
      field("product.code", "商品编码", "product", "商品信息", "P-M1-001"),
      field("products.0.expiry_date", "有效期至", "product", "商品信息", "2028-01-01"),
      field("receiving.actual_qty", "实收数量", "receiving", "收货信息", "120"),
      field("receiving.arrival_temperature_celsius", "到货温度", "receiving", "收货信息", "5"),
      field("receiving.temperature_control_method", "温控方式", "receiving", "收货信息", "冷藏"),
      field("receiving.transport_duration_minutes", "运输时长（分钟）", "receiving", "收货信息", "120"),
      field("receiving.vehicle_no", "车牌号", "receiving", "收货信息", "沪A-12345"),
      field("receiving.carrier", "承运商", "receiving", "收货信息", "华东冷链承运商"),
      field("inspection.conclusion", "验收结论", "inspection", "验收信息", "合格"),
      field("inspection.first_signer_id", "第一签字人", "inspection", "验收信息", "收货员0101"),
      field("inspection.second_signer_id", "第二签字人", "inspection", "验收信息", "收货员0102"),
    ];
  }
  if (libraryCode === "m2_acceptance_record") {
    return [
      field("asn.code", "ASN 号", "order", "订单信息", "ASN-202607070001"),
      field("supplier.name", "供应商", "order", "订单信息", "华东医药供应链"),
      field("products.0.batch_no", "批号", "product", "商品信息", "BATCH-202606"),
      field("products.0.expiry_date", "有效期至", "product", "商品信息", "2028-01-01"),
      field("receiving.actual_qty", "实收数量", "receiving", "收货信息", "120"),
      field("inspection.conclusion", "验收结论", "inspection", "验收信息", "合格"),
      field("inspection.first_signer_id", "第一签字人", "inspection", "验收信息", "收货员0101"),
      field("inspection.second_signer_id", "第二签字人", "inspection", "验收信息", "收货员0102"),
    ];
  }
  if (libraryCode.includes("location")) {
    return [
      field("location.code", "库位编码", "location", "库位信息", "A01-01-02-03"),
      field("location.zone", "库区", "location", "库位信息", "A01"),
    ];
  }
  if (libraryCode.includes("lpn")) {
    return [field("lpn.code", "LPN", "lpn", "LPN 信息", "LPN-202607070001")];
  }
  return [
    field("asn.code", "业务单号", "order", "单据信息", "DOC-202607070001"),
    field("product.name", "商品名称", "product", "商品信息", "冷藏胰岛素注射液"),
  ];
}

function fieldCount(libraryCode: string) {
  if (libraryCode.includes("label")) return 8;
  if (libraryCode.includes("acceptance")) return 24;
  return 16;
}

function field(
  fieldPath: string,
  displayName: string,
  groupCode: string,
  groupName: string,
  sampleValue: string,
) {
  return { fieldPath, displayName, groupCode, groupName, sampleValue };
}

function sourceSchema(libraryCode: string) {
  if (libraryCode.startsWith("m2_")) return "ReceivingOrder";
  if (libraryCode.startsWith("m4_")) return "OutboundOrder";
  if (libraryCode.startsWith("m3_")) return "InventoryBatch";
  return "MasterData";
}

function hiprintJson(fieldPath: string) {
  return {
    panels: [{
      index: 0,
      paperType: "A4",
      width: 210,
      height: 297,
      orient: "portrait",
      printElements: [{
        options: { field: fieldPath, title: "业务单号", left: 20, top: 20, width: 260, height: 20 },
        printElementType: { type: "text" },
      }],
    }],
  };
}
