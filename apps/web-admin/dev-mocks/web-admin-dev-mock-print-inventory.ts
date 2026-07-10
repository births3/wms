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
  if (req.method === "GET" && pathname === "/api/v1/print-templates/field-libraries") {
    const data = fieldLibraries();
    sendJson(res, 200, { data, page: { count: data.length, next_cursor: null } });
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

function fieldLibraries() {
  return (devSystemDictionaryItemsByCode.print_template_type ?? []).map((type, index) => {
    const libraryCode = String(type.params.field_library_code ?? type.item_code);
    return {
      id: `00000000-0000-0000-0000-0000000028${String(index + 1).padStart(2, "0")}`,
      library_code: libraryCode,
      library_name: `${type.item_name}字段库`,
      source_schema: sourceSchema(libraryCode),
      latest_version_id: `00000000-0000-0000-0000-0000000029${String(index + 1).padStart(2, "0")}`,
      version_no: 1,
      field_count: fieldCount(libraryCode),
      created_at: type.created_at,
      published_at: type.updated_at,
      published_by: devUserId,
    };
  });
}

function fieldDefinitions(versionId: string) {
  const library = fieldLibraries().find((item) => item.latest_version_id === versionId);
  const libraryCode = library?.library_code ?? "m2_asn";
  return fieldPaths(libraryCode).map((field, index) => ({
    id: `00000000-0000-0000-0000-000000004${String(index + 1).padStart(3, "0")}`,
    library_version_id: versionId,
    field_path: field.fieldPath,
    field_type: "string",
    source_schema: sourceSchema(libraryCode),
    display_name: field.displayName,
    group_code: field.groupCode,
    group_name: field.groupName,
    metadata: { printable: true, sensitive: false, sample_value: field.sampleValue },
    sort_order: (index + 1) * 10,
  }));
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
      field("product.name", "商品名称", "product", "商品信息", "冷藏胰岛素注射液"),
      field("product.code", "商品编码", "product", "商品信息", "P-M1-001"),
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
