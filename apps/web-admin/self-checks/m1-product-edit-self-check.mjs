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
      attrs: { storage_condition: "cold", source: "erp" },
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

  const request = productEditRequestFromForm({
    ...form,
    productCode: "P-M1-SHOULD-NOT-UPDATE",
    productName: "冷藏胰岛素注射液-更新",
    storageCondition: "normal",
  });
  assert.equal("product_code" in request, false);
  assert.equal(request.product_name, "冷藏胰岛素注射液-更新");
  assert.equal(request.attrs.storage_condition, "normal");
  assert.equal(request.attrs.source, "erp");
} finally {
  await server.close();
}
