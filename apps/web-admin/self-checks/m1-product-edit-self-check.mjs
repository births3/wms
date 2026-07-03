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
  const { productEditFormFromRow, productEditRequestFromForm } = await server.ssrLoadModule(
    "/src/pages/master-data/m1-product-edit-model.ts",
  );

  const row = {
    id: "00000000-0000-0000-0000-000000001001",
    code: "P-M1-001",
    name: "冷藏胰岛素注射液",
    status: "active",
    statusLabel: "启用",
    ownerId: "00000000-0000-0000-0000-000000000001",
    primaryLabel: "规格",
    primaryValue: "10ml*1支",
    secondaryLabel: "批准文号",
    secondaryValue: "国药准字H20260001",
    extraLabel: "储存条件",
    extraValue: "cold",
    createdAt: "2026-06-29T00:00:00.000Z",
    updatedAt: "2026-06-29T00:00:00.000Z",
    productFields: {
      approvalNo: "国药准字H20260001",
      attrs: {
        large_package: "20 件/大包",
        middle_package: "10 件/中包",
        source: "erp",
        storage_condition: "cold",
        unit_height_mm: "30",
        unit_length_mm: "120",
        unit_volume_cm3: "360",
        unit_weight_g: "180",
        unit_width_mm: "100",
      },
      dosageForm: "注射剂",
      manufacturer: "鹏鹞示例药业",
      specialDrugCategoryCode: "none",
      spec: "10ml*1支",
      storageCondition: "cold",
    },
    searchText: "",
  };

  const form = productEditFormFromRow(row);
  assert.equal(form.productCode, "P-M1-001");
  assert.equal(form.productName, "冷藏胰岛素注射液");
  assert.equal(form.storageCondition, "cold");
  assert.equal(form.middlePackage, "10 件/中包");
  assert.equal(form.largePackage, "20 件/大包");
  assert.equal(form.unitLengthMm, "120");
  assert.equal(form.unitWidthMm, "100");
  assert.equal(form.unitHeightMm, "30");
  assert.equal(form.unitWeightG, "180");
  assert.equal(form.unitVolumeCm3, "360");

  const request = productEditRequestFromForm({
    ...form,
    productCode: "P-M1-SHOULD-NOT-UPDATE",
    productName: "冷藏胰岛素注射液-更新",
    storageCondition: "normal",
    unitWeightG: "200",
  });
  assert.equal("product_code" in request, false);
  assert.equal(request.product_name, "冷藏胰岛素注射液-更新");
  assert.equal(request.attrs.storage_condition, "normal");
  assert.equal(request.attrs.source, "erp");
  assert.equal(request.attrs.middle_package, "10 件/中包");
  assert.equal(request.attrs.large_package, "20 件/大包");
  assert.equal(request.attrs.unit_length_mm, "120");
  assert.equal(request.attrs.unit_width_mm, "100");
  assert.equal(request.attrs.unit_height_mm, "30");
  assert.equal(request.attrs.unit_weight_g, "200");
  assert.equal(request.attrs.unit_volume_cm3, "360");
} finally {
  await server.close();
}
