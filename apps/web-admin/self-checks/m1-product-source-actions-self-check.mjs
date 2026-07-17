import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("..", import.meta.url)),
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});

try {
  const apiSource = readFileSync(
    fileURLToPath(new URL("../src/features/master-data/master-data-queries/api.ts", import.meta.url)),
    "utf8",
  );
  const { productSourceLabel, masterDataActionLabels, productTableClassName, masterDataColumns } =
    await server.ssrLoadModule("/src/pages/master-data/m1-product-page-model.ts");
  const { baseMasterDataColumns } = await server.ssrLoadModule(
    "/src/pages/master-data/M1MasterDataColumns.tsx",
  );
  const { productColumns } = await server.ssrLoadModule("/src/pages/master-data/ProductEditTable.tsx");
  const {
    productRow,
    supplierRow,
    customerRow,
    warehouseRow,
    locationRow,
    storageConditionDisplayLabel,
  } = await server.ssrLoadModule("/src/features/master-data/master-data-queries.ts");
  const { crudTargetForRow } = await server.ssrLoadModule(
    "/src/pages/master-data/MasterDataCrudDialog.tsx",
  );
  const { parseSupplierImportText } = await server.ssrLoadModule(
    "/src/pages/master-data/MasterDataSourceActions.tsx",
  );
  const { isValidUnifiedSocialCreditCode, validateSupplierQualificationFields } = await server.ssrLoadModule(
    "/src/pages/master-data/supplier-qualification-validation.ts",
  );
  assert.match(apiSource, /web-m1-product-create/);
  assert.match(apiSource, /web-m1-product-update/);
  assert.match(apiSource, /web-m1-supplier-create/);
  assert.match(apiSource, /web-m1-supplier-update-/);
  assert.match(apiSource, /web-m1-customer-create/);
  assert.match(apiSource, /web-m1-customer-update-/);
  assert.match(apiSource, /web-m1-warehouse-create/);
  assert.match(apiSource, /web-m1-warehouse-update-/);

  assert.equal(productSourceLabel("manual"), "手工新建");
  assert.equal(productSourceLabel("batch_import"), "批量导入");
  assert.equal(productSourceLabel("api_import"), "API接口导入");
  assert.equal(productSourceLabel("erp"), "API接口导入");
  assert.equal(productSourceLabel(undefined), "-");
  assert.deepEqual(masterDataActionLabels("m1-products"), ["新建商品", "批量导入"]);
  assert.deepEqual(masterDataActionLabels("m1-business-partners"), [
    "新建供应商",
    "导入供应商",
    "新建客户",
    "导入客户",
  ]);
  assert.equal(productTableClassName("m1-products"), "min-w-[2380px]");

  const row = productRow({
    id: "00000000-0000-0000-0000-000000001001",
    owner_id: "00000000-0000-0000-0000-000000000001",
    product_code: "P-M1-001",
    product_name: "冷藏胰岛素注射液",
    approval_no: "国药准字H20260001",
    spec: "10ml*1支",
    dosage_form: "注射剂",
    manufacturer: "鹏鹞示例药业",
    special_drug_category_code: "none",
    status: "active",
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
    },
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  });

  assert.equal(row.secondaryLabel, "批准文号");
  assert.equal(row.secondaryValue, "国药准字H20260001");
  assert.equal(row.extraValue, "冷藏");
  assert.equal(row.productFields.storageCondition, "cold");
  assert.equal(storageConditionDisplayLabel("cold"), "冷藏");
  assert.equal(storageConditionDisplayLabel("normal"), "常温");
  assert.equal(row.sourceValue, "API接口导入");
  assert.match(row.searchText, /api接口导入/i);
  assert.equal(row.productFields.middlePackage, "10 件/中包");
  assert.equal(row.productFields.largePackage, "20 件/大包");
  assert.equal(row.productFields.unitLengthMm, "120");
  assert.equal(row.productFields.unitWidthMm, "100");
  assert.equal(row.productFields.unitHeightMm, "30");
  assert.equal(row.productFields.unitWeightG, "180");
  assert.equal(row.productFields.unitVolumeCm3, "360");
  assert.match(row.searchText, /10 件\/中包/);
  assert.match(row.searchText, /180/);

  const warehouse = warehouseRow({
    id: "00000000-0000-0000-0000-000000003001",
    owner_id: "00000000-0000-0000-0000-000000000001",
    warehouse_code: "WH-M1-001",
    warehouse_name: "鹏鹞冷链仓",
    warehouse_type: "physical",
    status: "active",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  });
  assert.equal(warehouse.code, "WH-M1-001");
  assert.equal(warehouse.name, "鹏鹞冷链仓");
  assert.equal(warehouse.secondaryValue, "物理仓");
  assert.equal(warehouse.warehouseFields.warehouseType, "physical");
  assert.notEqual(warehouse.primaryValue, warehouse.id);
  assert.doesNotMatch(warehouse.primaryValue, /^[0-9a-f-]{36}$/i);

  const warehouseRefs = new Map([
    [
      "00000000-0000-0000-0000-000000003001",
      { id: "00000000-0000-0000-0000-000000003001", code: "WH-M1-001", name: "鹏鹞冷链仓" },
    ],
  ]);
  const location = locationRow(
    {
      id: "00000000-0000-0000-0000-000000000201",
      owner_id: "00000000-0000-0000-0000-000000000001",
      warehouse_id: "00000000-0000-0000-0000-000000003001",
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
    },
    new Map([["storage", "存储位"]]),
    warehouseRefs,
  );
  assert.equal(location.locationFields.warehouse, "WH-M1-001 · 鹏鹞冷链仓");
  assert.equal(location.locationFields.warehouseId, "00000000-0000-0000-0000-000000003001");
  assert.equal(location.locationFields.zone, "A01");
  assert.equal(location.locationFields.zoneId, "00000000-0000-0000-0000-000000003101");
  assert.doesNotMatch(location.locationFields.warehouse, /^[0-9a-f-]{36}$/i);
  assert.doesNotMatch(location.locationFields.zone, /^[0-9a-f-]{36}$/i);

  const productBaseColumns = masterDataColumns("m1-products", baseMasterDataColumns, []);
  assert.equal(productBaseColumns.find((column) => column.key === "primary")?.header, "规格");
  assert.equal(productBaseColumns.find((column) => column.key === "secondary")?.header, "批准文号");
  assert.equal(productBaseColumns.find((column) => column.key === "extra")?.header, "储存条件");
  const businessPartnerBaseColumns = masterDataColumns("m1-business-partners", baseMasterDataColumns, []);
  assert.equal(businessPartnerBaseColumns.find((column) => column.key === "primary")?.header, "统一代码 / 资质证号");
  assert.equal(businessPartnerBaseColumns.find((column) => column.key === "secondary")?.header, "联系人 / 类型");
  assert.equal(businessPartnerBaseColumns.find((column) => column.key === "extra")?.header, "档案类型 / 货主");
  const warehouseColumns = masterDataColumns("m1-warehouses", baseMasterDataColumns, []);
  assert.equal(warehouseColumns.find((column) => column.key === "primary")?.header, "货主");
  assert.equal(warehouseColumns.find((column) => column.key === "secondary")?.header, "仓库类型");
  assert.equal(warehouseColumns.find((column) => column.key === "extra")?.header, "仓库名称");
  const zoneColumns = masterDataColumns("m1-zones", baseMasterDataColumns, []);
  assert.equal(zoneColumns.find((column) => column.key === "primary")?.header, "仓库");
  assert.equal(zoneColumns.find((column) => column.key === "secondary")?.header, "库区");
  assert.equal(zoneColumns.find((column) => column.key === "extra")?.header, "库位数");

  const sourceColumn = masterDataColumns("m1-products", [], []).find(
    (column) => column.key === "source",
  );
  assert.equal(sourceColumn?.render?.(row), "API接口导入");
  const columns = productColumns(masterDataColumns("m1-products", [], []), () => undefined);
  assert.ok(columns.some((column) => column.key === "productPackaging"));
  assert.ok(columns.some((column) => column.key === "unitSize"));
  assert.ok(columns.some((column) => column.key === "unitWeightVolume"));

  const supplier = supplierRow({
    id: "00000000-0000-0000-0000-000000002001",
    owner_id: "00000000-0000-0000-0000-000000000001",
    supplier_code: "S-M1-001",
    supplier_name: "配送供应商A",
    license_no: "SPL-001",
    contact_name: "王供应",
    status: "active",
    source: "manual",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  });
  const customer = customerRow({
    id: "00000000-0000-0000-0000-000000003001",
    owner_id: "00000000-0000-0000-0000-000000000001",
    customer_code: "C-M1-001",
    customer_name: "连锁门店A",
    license_no: "CUS-001",
    status: "active",
    source: "batch_import",
    created_at: "2026-06-29T00:00:00.000Z",
    updated_at: "2026-06-29T00:00:00.000Z",
  });
  assert.equal(supplier.partnerKind, "supplier");
  assert.equal(supplier.partnerTypeLabel, "供应商");
  assert.match(supplier.searchText, /供应商/);
  assert.equal(customer.partnerKind, "customer");
  assert.equal(customer.partnerTypeLabel, "客户\/门店");
  assert.match(customer.searchText, /客户\/门店/);
  const businessPartnerColumns = masterDataColumns("m1-business-partners", [], []);
  assert.ok(businessPartnerColumns.some((column) => column.key === "businessPartnerType"));
  assert.ok(businessPartnerColumns.some((column) => column.key === "source"));
  assert.equal(
    businessPartnerColumns.find((column) => column.key === "businessPartnerType")?.render?.(supplier),
    "供应商",
  );
  assert.equal(crudTargetForRow("m1-business-partners", supplier).kind, "supplier");
  assert.equal(crudTargetForRow("m1-business-partners", customer).kind, "customer");
  assert.equal(isValidUnifiedSocialCreditCode("91350100M000100Y43"), true);
  assert.equal(isValidUnifiedSocialCreditCode("91350100M000100Y49"), false);
  assert.equal(isValidUnifiedSocialCreditCode("INVALID-USCC"), false);
  assert.equal(
    parseSupplierImportText("supplier_code,supplier_name,license_no,contact_name\nS-001,供应商A,91350100M000100Y43,王供应")[0].license_no,
    "91350100M000100Y43",
  );
  assert.throws(
    () => parseSupplierImportText("supplier_code,supplier_name,license_no,contact_name\nS-001,供应商A,INVALID-USCC,王供应"),
    /统一社会信用代码格式错误/,
  );
  assert.throws(
    () => validateSupplierQualificationFields({ unifiedSocialCreditCode: "INVALID-USCC", contactName: "王供应" }),
    /统一社会信用代码格式错误/,
  );
} finally {
  await server.close();
}
