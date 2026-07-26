import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/shell-dev/screenshots");
const inboundDocumentsArtifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m2-inbound-documents");

test("dev mock 保留主数据分页和入库动作状态", async ({ request }) => {
  const productsResponse = await request.get("/api/v1/master-data/products");
  expect(productsResponse.status()).toBe(200);
  const products = await productsResponse.json() as { data: unknown[]; page: { count: number } };
  expect(products.page.count).toBe(products.data.length);
  expect(products.data.length).toBeGreaterThanOrEqual(3);
  const productRows = products.data as Array<{
    id: string;
    product_code: string;
    product_name: string;
    spec: string | null;
    dosage_form: string | null;
    approval_no: string | null;
    manufacturer: string | null;
    special_drug_category_code: string | null;
    attrs: Record<string, unknown>;
    status: string;
  }>;
  const baseProduct = productRows.find((product) => product.product_code === "P-M1-001");
  expect(baseProduct).toBeDefined();
  const updatePayload = {
    product_name: baseProduct?.product_name,
    spec: baseProduct?.spec,
    dosage_form: baseProduct?.dosage_form,
    approval_no: baseProduct?.approval_no,
    manufacturer: baseProduct?.manufacturer,
    special_drug_category_code: "controlled",
    attrs: { ...baseProduct?.attrs, storage_condition: "frozen" },
    status: "inactive",
  };
  const updateResponse = await request.patch(`/api/v1/master-data/products/${baseProduct?.id}`, {
    data: updatePayload,
  });
  expect(updateResponse.status()).toBe(200);
  await expect.poll(async () => {
    const productsAfterUpdate = await request.get("/api/v1/master-data/products");
    const updatedRows = (await productsAfterUpdate.json() as { data: typeof productRows }).data;
    return updatedRows.find((product) => product.product_code === "P-M1-001");
  }).toMatchObject({
    special_drug_category_code: "controlled",
    attrs: { storage_condition: "frozen" },
    status: "inactive",
  });
  await request.patch(`/api/v1/master-data/products/${baseProduct?.id}`, {
    data: {
      ...updatePayload,
      special_drug_category_code: baseProduct?.special_drug_category_code,
      attrs: baseProduct?.attrs,
      status: baseProduct?.status,
    },
  });

  const secondProduct = productRows.find((product) => product.product_code === "P-M1-002");
  expect(secondProduct).toBeDefined();
  const secondUpdateResponse = await request.patch(`/api/v1/master-data/products/${secondProduct?.id}`, {
    data: { ...secondProduct, status: "inactive" },
  });
  expect(secondUpdateResponse.status()).toBe(200);
  await expect.poll(async () => {
    const response = await request.get("/api/v1/master-data/products");
    const rows = (await response.json() as { data: typeof productRows }).data;
    return rows.find((product) => product.product_code === "P-M1-002")?.status;
  }).toBe("inactive");
  await request.patch(`/api/v1/master-data/products/${secondProduct?.id}`, {
    data: secondProduct,
  });

  const createdProductResponse = await request.post("/api/v1/master-data/products", {
    data: { product_code: "P-M1-E2E", product_name: "默认值商品", attrs: {} },
  });
  await expect(createdProductResponse.json()).resolves.toMatchObject({
    attrs: { source: "api_import", storage_condition: "normal" },
    status: "active",
  });
  const createdSupplierResponse = await request.post("/api/v1/master-data/suppliers", {
    data: { supplier_code: "S-M1-E2E", supplier_name: "默认值供应商" },
  });
  await expect(createdSupplierResponse.json()).resolves.toMatchObject({ source: "api_import", status: "active" });
  const createdCustomerResponse = await request.post("/api/v1/master-data/customers", {
    data: { customer_code: "C-M1-E2E", customer_name: "默认值客户" },
  });
  await expect(createdCustomerResponse.json()).resolves.toMatchObject({ source: "api_import", status: "active" });

  const boundOwnerId = "00000000-0000-0000-0000-000000000001";
  const createdLocationResponse = await request.post("/api/v1/master-data/locations", {
    data: {
      warehouse_id: "00000000-0000-0000-0000-000000003001",
      zone_id: "00000000-0000-0000-0000-000000003101",
      location_code: "A01-E2E-01-01",
      row_no: 1,
      column_no: 1,
      layer_no: 1,
      max_volume_cm3: 100000,
      max_sku_count: 1,
      location_type: "storage",
      bound_owner_id: boundOwnerId,
    },
  });
  const createdLocation = await createdLocationResponse.json() as { id: string };
  const invalidLocationResponse = await request.post("/api/v1/master-data/locations", {
    data: {
      warehouse_id: "not-a-uuid",
      zone_id: "00000000-0000-0000-0000-000000003101",
      location_code: "A01-INVALID-01-01",
      row_no: 1.5,
      column_no: 1,
      layer_no: 1,
      max_volume_cm3: Number.MAX_SAFE_INTEGER + 1,
      max_sku_count: 2_147_483_648,
      location_type: "storage",
    },
  });
  expect(invalidLocationResponse.status()).toBe(422);
  const updatedLocationResponse = await request.patch(`/api/v1/master-data/locations/${createdLocation.id}`, {
    data: { status: "disabled" },
  });
  await expect(updatedLocationResponse.json()).resolves.toMatchObject({ bound_owner_id: boundOwnerId });

  const batchRequest = {
      warehouse_id: "00000000-0000-0000-0000-000000003001",
      zone_id: "00000000-0000-0000-0000-000000003101",
      area_code: "E2E",
      row_start: 2,
      row_end: 2,
      column_start: 1,
      column_end: 2,
      layer_start: 1,
      layer_end: 1,
      max_volume_cm3: 100000,
      max_sku_count: 1,
      location_type: "storage",
  };
  const missingBatchIdempotency = await request.post("/api/v1/master-data/locations/batch-create", {
    data: { ...batchRequest, area_code: "MIS", row_start: 3, row_end: 3 },
  });
  expect(missingBatchIdempotency.status()).toBe(400);
  await expect(missingBatchIdempotency.json()).resolves.toMatchObject({ code: "M1_IDEMPOTENCY_REQUIRED" });
  const batchLocationResponse = await request.post("/api/v1/master-data/locations/batch-create", {
    headers: { "Idempotency-Key": "m1-e2e-location-batch" },
    data: batchRequest,
  });
  await expect(batchLocationResponse.json()).resolves.toMatchObject({
    data: [
      { location_code: "E2E-02-01-01" },
      { location_code: "E2E-02-02-01" },
    ],
    page: { count: 2, next_cursor: null },
  });
  const replayBatchResponse = await request.post("/api/v1/master-data/locations/batch-create", {
    headers: { "Idempotency-Key": "m1-e2e-location-batch" },
    data: batchRequest,
  });
  expect(replayBatchResponse.status()).toBe(200);
  await expect(replayBatchResponse.json()).resolves.toMatchObject(await batchLocationResponse.json());
  const conflictingBatchResponse = await request.post("/api/v1/master-data/locations/batch-create", {
    headers: { "Idempotency-Key": "m1-e2e-location-batch" },
    data: { ...batchRequest, area_code: "CNF" },
  });
  expect(conflictingBatchResponse.status()).toBe(409);
  await expect(conflictingBatchResponse.json()).resolves.toMatchObject({ code: "M1_IDEMPOTENCY_CONFLICT" });
  const locationsAfterBatch = await request.get("/api/v1/master-data/locations");
  const locationCodes = (await locationsAfterBatch.json() as { data: Array<{ location_code: string }> })
    .data.map((location) => location.location_code);
  expect(locationCodes).toEqual(expect.arrayContaining(["E2E-02-01-01", "E2E-02-02-01"]));

  const duplicateBatchResponse = await request.post("/api/v1/master-data/locations/batch-create", {
    headers: { "Idempotency-Key": "m1-e2e-location-batch-duplicate" },
    data: batchRequest,
  });
  expect(duplicateBatchResponse.status()).toBe(409);
  await expect(duplicateBatchResponse.json()).resolves.toMatchObject({ code: "M1_LOCATION_DUPLICATE" });

  for (const invalidPayload of [
    {
      area_code: "MAX",
      row_start: 1,
      row_end: 99,
      column_start: 1,
      column_end: 6,
      layer_start: 1,
      layer_end: 1,
      max_volume_cm3: 100000,
      max_sku_count: 1,
      location_type: "storage",
    },
    {
      area_code: "CAP",
      row_start: 1,
      row_end: 1,
      column_start: 1,
      column_end: 1,
      layer_start: 1,
      layer_end: 1,
      max_volume_cm3: -1,
      max_sku_count: 0,
      location_type: "unknown",
    },
    {
      area_code: "DEC",
      row_start: 1.5,
      row_end: 2,
      column_start: 1,
      column_end: 1,
      layer_start: 1,
      layer_end: 1,
      max_volume_cm3: 100000.5,
      max_sku_count: 1.5,
      location_type: "storage",
    },
    { ...batchRequest, area_code: "UID", warehouse_id: "not-a-uuid" },
    { ...batchRequest, area_code: "ZON", zone_id: "00000000-0000-0000-0000-000000003199" },
    { ...batchRequest, area_code: "OWN", bound_owner_id: "00000000-0000-0000-0000-000000000999" },
    { ...batchRequest, area_code: "I32", max_sku_count: 2_147_483_648 },
    { ...batchRequest, area_code: "I64", max_volume_cm3: Number.MAX_SAFE_INTEGER + 1 },
  ]) {
    const invalidBatchResponse = await request.post("/api/v1/master-data/locations/batch-create", {
      headers: { "Idempotency-Key": `m1-e2e-invalid-${invalidPayload.area_code}` },
      data: {
        warehouse_id: "00000000-0000-0000-0000-000000003001",
        zone_id: "00000000-0000-0000-0000-000000003101",
        ...invalidPayload,
      },
    });
    expect(invalidBatchResponse.status()).toBe(422);
    await expect(invalidBatchResponse.json()).resolves.toMatchObject({ code: "M1_LOCATION_BATCH_INVALID" });
  }

  const missingDocumentTypeResponse = await request.post("/api/v1/inbound/receiving-orders", {
    data: { receipt_no: "ASN-MISSING-DOCUMENT-TYPE", lines: [] },
  });
  expect(missingDocumentTypeResponse.status()).toBe(422);
  await expect(missingDocumentTypeResponse.json()).resolves.toMatchObject({
    code: "W3-422",
    severity: "error",
    details: {},
    trace_id: "dev-mock",
  });
  const unknownRouteResponse = await request.get("/api/v1/dev-mock-contract-probe");
  expect(unknownRouteResponse.status()).toBe(404);
  await expect(unknownRouteResponse.json()).resolves.toMatchObject({
    code: "DEV_MOCK_NOT_FOUND",
    severity: "error",
    details: {},
    trace_id: "dev-mock",
  });

  const ordersResponse = await request.get("/api/v1/inbound/receiving-orders");
  expect(ordersResponse.status()).toBe(200);
  const orders = await ordersResponse.json() as {
    data: Array<{ id: string; status: string }>;
    page: { count: number };
  };
  expect(orders.page.count).toBe(100);
  const released = orders.data.find((order) => order.status === "released");
  expect(released).toBeDefined();

  const receiveResponse = await request.post(`/api/v1/inbound/receiving-orders/${released?.id}/receive`, {
    data: { actual_qty: 20, shortage_qty: 0, rejected_qty: 0 },
  });
  expect(receiveResponse.status()).toBe(200);
  const receipt = await receiveResponse.json() as { id: string };
  expect(receipt.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  const inspectResponse = await request.post(`/api/v1/inbound/receiving-orders/${released?.id}/inspect`, {
    data: { batch_no: "BATCH-E2E", accepted_qty: 20, rejected_qty: 0, quality_status: "qualified" },
  });
  expect(inspectResponse.status()).toBe(200);
  const inspection = await inspectResponse.json() as { id: string };
  expect(inspection.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  const detailResponse = await request.get(`/api/v1/inbound/receiving-orders/${released?.id}`);
  expect(detailResponse.status()).toBe(200);
  await expect(detailResponse.json()).resolves.toMatchObject({ status: "inspecting" });
});

test("dev mock 保留 H9 字段库数量和 M2 ASN 字段", async ({ request }) => {
  const librariesResponse = await request.get("/api/v1/print-templates/field-libraries");
  expect(librariesResponse.status()).toBe(200);
  const libraries = await librariesResponse.json() as {
    data: Array<{ library_code: string; latest_version_id: string; field_count: number }>;
  };
  const asnLibrary = libraries.data.find((library) => library.library_code === "m2_asn");
  expect(asnLibrary).toMatchObject({ field_count: 16 });

  const fieldsResponse = await request.get(
    `/api/v1/print-templates/field-libraries/${asnLibrary?.latest_version_id}/fields`,
  );
  expect(fieldsResponse.status()).toBe(200);
  const fields = await fieldsResponse.json() as { data: Array<{ field_path: string }> };
  expect(fields.data.map((field) => field.field_path)).toContain("product.code");
});

test("侧边栏筛选菜单支持 Escape 和点击页面内容关闭", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  const pageHeading = page.getByRole("heading", { name: "运营总览" });
  await expect(pageHeading).toBeVisible();

  await page.getByRole("button", { name: "筛选菜单" }).click();
  const menuFilter = page.getByLabel("筛选菜单");
  await expect(menuFilter).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(menuFilter).toBeHidden();

  await page.getByRole("button", { name: "筛选菜单" }).click();
  await expect(menuFilter).toBeVisible();

  await pageHeading.click();

  await expect(menuFilter).toBeHidden();
  await expect(page.getByRole("button", { name: "筛选菜单" })).toBeVisible();
});

test("展开菜单时折叠同级其他菜单", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  const dashboardSection = page.getByRole("button", { name: "工作台", exact: true });
  const inboundSection = page.getByRole("button", { name: "入库业务", exact: true });
  const masterSection = page.getByRole("button", { name: "基础档案", exact: true });
  await expect(dashboardSection).toHaveAttribute("aria-expanded", "true");
  await expect(inboundSection).toHaveAttribute("aria-expanded", "false");
  await inboundSection.click();
  await expect(dashboardSection).toHaveAttribute("aria-expanded", "false");
  await page.getByRole("button", { name: "入库作业", exact: true }).click();
  await masterSection.click();
  await expect(masterSection).toHaveAttribute("aria-expanded", "true");
  await expect(inboundSection).toHaveAttribute("aria-expanded", "false");
  await inboundSection.click();
  await expect(page.getByRole("button", { name: "入库作业", exact: true })).toHaveAttribute("aria-expanded", "false");
  await masterSection.click();

  const masterGroup = page.getByRole("button", { name: "主数据", exact: true });
  const storageGroup = page.getByRole("button", { name: "仓储资料", exact: true });
  await masterGroup.click();
  await storageGroup.click();
  await expect(storageGroup).toHaveAttribute("aria-expanded", "true");
  await expect(masterGroup).toHaveAttribute("aria-expanded", "false");
});

for (const target of [
  { section: "基础档案", group: "系统配置", id: "m1-system-dictionary", heading: "M1 系统字典" },
  { section: "入库业务", group: "入库作业", id: "m2-receiving", heading: "M2 收货管理" },
  { section: "库内业务", group: "库存管理", id: "m3-batches", heading: "M3 批号管理" },
  { section: "基础能力", group: "M-CG 编码能力", id: "mcg-numbering", heading: "M-CG 单据号规则" },
  { section: "出库业务", group: "出库作业", id: "m4-orders", heading: "M4 出库订单管理" },
  { section: "出库业务", group: "出库作业", id: "m4-waves", heading: "M4 波次规划" },
  { section: "出库业务", group: "出库作业", id: "m4-review", heading: "M4 复核发货" },
  { section: "出库业务", group: "出库作业", id: "m4-returns", heading: "M4 采购退货出库" },
]) {
  test(`${target.heading} 能通过三层菜单打开`, async ({ page }) => {
    await page.goto("/");
    await Promise.all([
      page.waitForResponse((response) => response.url().includes("/api/v1/admin/menus/published")),
      page.getByRole("button", { name: "登录" }).click(),
    ]);
    await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

    const sectionButton = page.getByRole("button", { name: target.section });
    if (await sectionButton.getAttribute("aria-expanded") !== "true") await sectionButton.click();
    const groupButton = page.getByRole("button", { name: target.group });
    if (await groupButton.getAttribute("aria-expanded") !== "true") await groupButton.click();
    await page.getByRole("button", { name: `${target.heading} ${target.id}`, exact: true }).click();

    await expect(page.getByRole("heading", { name: target.heading, exact: true })).toBeVisible();
    if (target.id.startsWith("m4-")) {
      fs.mkdirSync(artifactsDir, { recursive: true });
      await page.screenshot({ path: path.join(artifactsDir, `${target.id}.png`), fullPage: false });
    }
  });
}

test("H1 菜单管理能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H1 权限租户" }).click();
  await page.getByRole("button", { name: /H1 菜单管理/ }).click();

  const h1Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H1 菜单管理" }) });
  await expect(h1Page.getByText("菜单树")).toBeVisible();
  await expect(h1Page.getByText("节点配置")).toBeVisible();
  await expect(h1Page.getByRole("button", { name: "发布" })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h1-menu-management.png"), fullPage: false });
});

test("H2 H3 基础能力能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H2 审计能力" }).click();
  // 与快捷入口按钮区分：侧栏菜单 accessible name 含 view id
  await page.getByRole("button", { name: "H2 审计追踪 h2-audit-trail", exact: true }).click();

  const h2Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H2 审计追踪" }) });
  await expect(h2Page.getByRole("heading", { name: "H2 审计追踪" })).toBeVisible();
  await expect(h2Page.getByText(/GET \/api\/v1\/audit\/events/).first()).toBeVisible();
  await expect(h2Page.getByText("IP 地址")).toBeVisible();
  await h2Page.getByRole("button", { name: "展开" }).click();
  await h2Page.getByLabel("动作类型").fill("验收提交");
  await h2Page.getByLabel("关联资源").fill("PO-2026-0001");
  await h2Page.getByLabel("商品编码").fill("P-M1-001");
  await h2Page.getByLabel("批号").fill("BATCH-M3-202606-01");
  await h2Page.getByRole("button", { name: "查询" }).click();
  await expect(h2Page.getByText("验收提交").first()).toBeVisible();
  await expect(h2Page.getByText("192.168.124.25")).toBeVisible();
  await expect(h2Page.getByText(/验收中/)).toBeVisible();
  await expect(h2Page.getByText(/已验收/)).toBeVisible();
  await expect(h2Page.getByText(/P-M1-001/)).toBeVisible();
  await expect(h2Page.getByText(/BATCH-M3-202606-01/)).toBeVisible();
  await h2Page.getByRole("button", { name: "导出" }).click();
  const exportDialog = page.getByRole("dialog");
  await expect(exportDialog.getByRole("heading", { name: "导出列表" })).toBeVisible();
  await expect(exportDialog.getByText(/当前筛选结果共 1 条/)).toBeVisible();
  await exportDialog.getByRole("button", { name: "取消" }).click();
  await page.screenshot({ path: path.join(artifactsDir, "h2-audit-trail.png"), fullPage: false });

  await page.getByRole("button", { name: "H3 契约能力" }).click();
  await page.getByRole("button", { name: "H3 OpenAPI h3-api-contract", exact: true }).click();
  const h3Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H3 OpenAPI 契约" }) });
  await expect(h3Page.getByText(/GET \/openapi\.json/).first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h3-api-contract.png"), fullPage: false });
});

test("H5 快递对接能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H5 快递能力" }).click();
  await page.getByRole("button", { name: "H5 快递对接 h5-express", exact: true }).click();

  const h5Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H5 快递对接" }) });
  await expect(h5Page.getByRole("heading", { name: "快递商配置" })).toBeVisible();
  await expect(h5Page.getByRole("heading", { name: "快递选择规则" })).toBeVisible();
  await expect(h5Page.getByText("顺丰速运")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h5-express.png"), fullPage: false });
});

test("M-CG 单据号规则可查询并打开新增弹窗", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "M-CG 编码能力" }).click();
  await page.getByRole("button", { name: "M-CG 单据号规则 mcg-numbering", exact: true }).click();

  const pageRoot = page.locator("section").filter({ has: page.getByRole("heading", { name: "M-CG 单据号规则" }) });
  await expect(pageRoot.getByText("采购入库单号")).toBeVisible();
  await expect(pageRoot.getByRole("heading", { name: "生成记录" })).toBeVisible();
  await pageRoot.getByRole("button", { name: "新增" }).click();
  await expect(page.getByRole("dialog")).toContainText("新增单据号规则");
  await page.getByRole("button", { name: "取消" }).click();
  await page.screenshot({ path: path.join(artifactsDir, "mcg-numbering.png"), fullPage: false });
});

test("入库资料录入使用开发 Mock 查询并上传上游随货同行单", async ({ page }) => {
  fs.mkdirSync(inboundDocumentsArtifactsDir, { recursive: true });
  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "入库业务", exact: true }).click();
  await page.getByRole("button", { name: "入库资料", exact: true }).click();
  for (const menuName of [
    "入库资料录入 m2-inbound-documents",
    "药检单审核 m-di-review",
    "M-DI 药检平台 m-di-platforms",
    "药检图章配置 m-di-stamp",
  ]) {
    await expect(page.getByRole("button", { name: menuName, exact: true })).toBeVisible();
  }
  await page.getByRole("button", { name: "入库资料录入 m2-inbound-documents", exact: true }).click();

  const pageRoot = page.locator("section").filter({ has: page.getByRole("heading", { name: "入库资料录入" }) });
  await expect(pageRoot.getByLabel("实际收货时间开始")).not.toHaveValue("");
  await expect(pageRoot.getByRole("button", { name: /药检单不齐/ })).toBeVisible();
  await pageRoot.getByRole("button", { name: /药检单不齐/ }).click();
  await pageRoot.getByRole("button", { name: /上游随货同行单不齐/ }).click();
  await expect(pageRoot.getByText(/共 \d+ 个 ASN/)).toBeVisible();

  await pageRoot.getByRole("button", { name: "录入资料" }).first().click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("tab", { name: "上游随货同行单" }).click();
  await dialog.locator('input[type="file"]').setInputFiles({
    name: "upstream-delivery.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.4 mock"),
  });
  const reason = dialog.getByLabel("修改原因");
  if (await reason.count()) await reason.fill("补充供应商盖章版本");
  await dialog.getByRole("button", { name: "上传并完成录入" }).click();
  await expect(dialog.getByText(/已关联 \d+ 个 ASN/)).toBeVisible();
  await dialog.evaluate((element) => { element.scrollTop = 0; });
  await page.screenshot({
    path: path.join(inboundDocumentsArtifactsDir, "inbound-documents-uploaded.png"),
    fullPage: false,
  });
});
