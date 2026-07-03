import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const server = await createServer({
  root: fileURLToPath(new URL("..", import.meta.url)),
  logLevel: "silent",
  server: { middlewareMode: true },
  appType: "custom",
});

try {
  const { productSourceLabel, masterDataActionLabels, productTableClassName, masterDataColumns } =
    await server.ssrLoadModule("/src/pages/master-data/m1-product-page-model.ts");
  const { productColumns } = await server.ssrLoadModule("/src/pages/master-data/ProductEditTable.tsx");
  const { productRow } = await server.ssrLoadModule(
    "/src/features/master-data/master-data-queries.ts",
  );

  assert.equal(productSourceLabel("manual"), "手工新建");
  assert.equal(productSourceLabel("batch_import"), "批量导入");
  assert.equal(productSourceLabel("api_import"), "API接口导入");
  assert.equal(productSourceLabel("erp"), "API接口导入");
  assert.equal(productSourceLabel(undefined), "-");
  assert.deepEqual(masterDataActionLabels("m1-products"), ["新建商品", "批量导入"]);
  assert.deepEqual(masterDataActionLabels("m1-suppliers"), ["新建供应商", "批量导入"]);
  assert.deepEqual(masterDataActionLabels("m1-customers"), ["新建客户", "批量导入"]);
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

  const sourceColumn = masterDataColumns("m1-products", [], []).find(
    (column) => column.key === "source",
  );
  assert.equal(sourceColumn?.render?.(row), "API接口导入");
  const columns = productColumns(masterDataColumns("m1-products", [], []), () => undefined);
  assert.ok(columns.some((column) => column.key === "productPackaging"));
  assert.ok(columns.some((column) => column.key === "unitSize"));
  assert.ok(columns.some((column) => column.key === "unitWeightVolume"));

  assert.ok(masterDataColumns("m1-suppliers", [], []).some((column) => column.key === "source"));
  assert.ok(masterDataColumns("m1-customers", [], []).some((column) => column.key === "source"));
} finally {
  await server.close();
}
