import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/m1-real/screenshots");
const productEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-products");
const businessPartnerEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-business-partners");
const featureFlagEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-feature-flags");
const systemDictionaryEvidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-system-dictionary");

test("M1 管理端读取真实后端数据", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();

  const cases = [
    { group: "主数据", menu: /M1 商品档案/, title: "M1 商品档案", text: "P-M1-E2E-001", shot: "products.png" },
    { group: "主数据", menu: /M1 客商档案/, title: "M1 客商档案", text: "S-M1-E2E-001", shot: "business-partners-supplier.png" },
    { group: "主数据", menu: /M1 客商档案/, title: "M1 客商档案", text: "C-M1-E2E-001", shot: "business-partners-customer.png" },
    { group: "仓储资料", menu: /M1 仓库管理/, title: "M1 仓库管理", text: "WH-M1-E2E-001", shot: "warehouses.png" },
    { group: "仓储资料", menu: /M1 库区管理/, title: "M1 库区管理", text: "A01", shot: "zones.png" },
    { group: "仓储资料", menu: /M1 库位管理/, title: "M1 库位管理", text: "A01-01-02-03", shot: "locations.png" },
    { group: "系统配置", menu: /M1 (功能开关|Feature Flag)/, title: "配置中心", text: "m3_inventory_batches_config_center_smoke", shot: "feature-flags.png" },
    { group: "系统配置", menu: /M1 系统字典/, title: "M1 系统字典", text: "purchase_inbound", shot: "dictionary.png" },
  ];

  for (const item of cases) {
    const menu = page.getByRole("navigation").getByRole("button", { name: item.menu });
    const group = page.getByRole("navigation").getByRole("button", { name: item.group, exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") {
      await group.click();
    }
    await expect(menu).toBeVisible();
    await menu.click();
    await expect(page.getByRole("heading", { name: item.title })).toBeVisible();
    if (item.title === "M1 系统字典") {
      const documentTypeButton = page.getByRole("button", { name: /单据类型 document_type/ });
      await documentTypeButton.click();
      await expect(page.getByRole("button", { name: "purchase_inbound", exact: true })).toBeVisible();
      await page.getByRole("button", { name: /特殊药品分类 special_drug_category/ }).click();
      await expect(page.getByRole("button", { name: "narcotic", exact: true })).toBeVisible();
      await expect(page.getByText("双人作业矩阵", { exact: true })).toBeVisible();
      await page.getByLabel("矩阵确认人").selectOption("00000000-0000-0000-0000-000000000103");
      const policySelect = page.getByLabel(/普通药品 入库 收货 双人策略/);
      const policyResponse = page.waitForResponse(
        (response) => response.url().includes("/api/v1/m-vr/dual-person-policy/rules") && response.request().method() === "PUT",
      );
      await policySelect.selectOption("dual_scan");
      const policyResponseValue = await policyResponse;
      expect(policyResponseValue.status(), await policyResponseValue.text()).toBe(200);
      await expect(policySelect).toHaveValue("dual_scan");
      fs.mkdirSync(systemDictionaryEvidenceDir, { recursive: true });
      await page.screenshot({
        path: path.join(systemDictionaryEvidenceDir, "special-drug-category.png"),
        fullPage: false,
      });
      await page.screenshot({
        path: path.join(systemDictionaryEvidenceDir, "dual-person-policy-matrix.png"),
        fullPage: false,
      });
    }
    if (item.title === "配置中心") {
      await page.getByRole("button", { name: "从文件源迁移" }).click();
    }
    if (item.title === "M1 仓库管理") {
      await expect(page.getByText("物理仓").first()).toBeVisible();
    }
    if (item.title !== "M1 系统字典") {
      await expect(page.getByText(item.text).first()).toBeVisible();
    }
    await page.screenshot({ path: path.join(artifactsDir, item.shot), fullPage: false });
    if (item.title === "配置中心") {
      fs.mkdirSync(featureFlagEvidenceDir, { recursive: true });
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({ path: path.join(featureFlagEvidenceDir, "feature-flags-current.png"), fullPage: false });
    }

    if (item.text === "C-M1-E2E-001") {
      const row = page.locator("tr", { hasText: item.text }).first();
      await row.getByRole("button", { name: "编辑", exact: true }).click();
      await expect(page.getByRole("heading", { name: "收货地址", exact: true })).toBeVisible();
      const profile = page.getByRole("region", { name: "客户门店信息" });
      await profile.getByLabel("档案类型").selectOption("store");
      await profile.getByRole("textbox", { name: "联系人", exact: true }).fill("E2E 联系人");
      await profile.getByRole("textbox", { name: "联系电话", exact: true }).fill("13800000001");
      await profile.getByRole("textbox", { name: "所属连锁", exact: true }).fill("E2E 连锁");
      await profile.getByRole("textbox", { name: "经营范围", exact: true }).fill("处方药, 医疗器械");
      await profile.getByRole("button", { name: "新增资质", exact: true }).click();
      const qualificationIndex = await profile.getByLabel(/资质类型-/).count();
      await profile.getByLabel(`资质类型-${qualificationIndex}`).fill("经营许可证");
      await profile.getByLabel(`资质编号-${qualificationIndex}`).fill("E2E-LIC-001");
      const profileResponse = page.waitForResponse((response) => response.url().includes("/profile") && response.request().method() === "PATCH");
      await profile.getByRole("button", { name: "保存档案", exact: true }).click();
      const profileResponseValue = await profileResponse;
      expect(profileResponseValue.status(), await profileResponseValue.text()).toBe(200);
      const address = page.getByRole("region", { name: "客户收货地址" });
      await address.getByLabel("省").fill("上海市");
      await address.getByLabel("市").fill("上海市");
      await address.getByLabel("区").fill("浦东新区");
      await address.getByLabel("详细地址").fill(`M1 E2E 地址 ${Date.now()}`);
      await address.getByRole("textbox", { name: "联系人", exact: true }).fill("E2E 收货联系人");
      await address.getByRole("textbox", { name: "联系电话", exact: true }).fill("13800000002");
      await address.getByRole("button", { name: "新增地址", exact: true }).last().click();
      await expect(address.getByText("E2E 收货联系人").first()).toBeVisible();
      await page.screenshot({
        path: path.join(artifactsDir, "business-partners-customer-address.png"),
        fullPage: false,
      });
      await page.getByRole("button", { name: "取消" }).click();
    }

    if (item.text === "S-M1-E2E-001") {
      const row = page.locator("tr", { hasText: item.text }).first();
      await row.getByRole("button", { name: "编辑", exact: true }).click();
      const supplierDialog = page.getByRole("dialog");
      await supplierDialog.getByLabel("统一社会信用代码").fill("INVALID-USCC");
      await expect(supplierDialog.getByRole("button", { name: "保存", exact: true })).toBeDisabled();
      await page.screenshot({
        path: path.join(artifactsDir, "business-partners-supplier-invalid-qualification.png"),
        fullPage: false,
      });
      await supplierDialog.getByLabel("统一社会信用代码").fill("91350100M000100Y43");
      await supplierDialog.getByLabel("联系人").fill("E2E 供应商联系人");
      const supplierUpdate = page.waitForResponse(
        (response) => response.url().includes("/api/v1/master-data/suppliers/") && response.request().method() === "PATCH",
      );
      await supplierDialog.getByRole("button", { name: "保存", exact: true }).click();
      const supplierUpdateResponse = await supplierUpdate;
      expect(supplierUpdateResponse.status(), await supplierUpdateResponse.text()).toBe(200);
      await expect(supplierDialog).toBeHidden();
      await expect(page.getByText("E2E 供应商联系人").first()).toBeVisible();
      await page.screenshot({
        path: path.join(artifactsDir, "business-partners-supplier-qualification-updated.png"),
        fullPage: false,
      });
    }

    if (item.title === "M1 商品档案") {
      await expect(page.getByText("ERP 权威商品投影")).toBeVisible();
      await expect(page.getByText(/本页只读/)).toBeVisible();
      await expect(page.getByRole("button", { name: "新增", exact: true })).toHaveCount(0);
      await expect(page.getByRole("button", { name: "导入", exact: true })).toHaveCount(0);
      const row = page.locator("tr", { hasText: "P-M1-E2E-001" }).first();
      await expect(row).toContainText("支 × 1");
      await expect(row).toContainText("盒 × 10");
      await expect(row).toContainText("06901234567891");
      await expect(row).toContainText("长 120 mm");
      await expect(row).toContainText("重量 180 g");
      fs.mkdirSync(productEvidenceDir, { recursive: true });
      await page.screenshot({
        path: path.join(productEvidenceDir, "product-read-only-contract-current.png"),
        fullPage: false,
      });
    }
  }

  const locationsMenu = page.getByRole("navigation").getByRole("button", { name: /M1 库位管理/ });
  if (!(await locationsMenu.isVisible())) await page.getByRole("button", { name: "仓储资料", exact: true }).click();
  await locationsMenu.click();
  await expect(page.getByRole("heading", { name: "M1 库位管理" })).toBeVisible();

  const marker = Date.now();
  const area = `Z${String((marker % 90) + 10)}`;
  const rowNo = ((Math.floor(marker / 100) % 90) + 10).toString();
  const columnNo = ((Math.floor(marker / 10_000) % 90) + 10).toString();
  const layerNo = ((Math.floor(marker / 1_000_000) % 90) + 10).toString();
  const createdCode = `${area}-${pad2(rowNo)}-${pad2(columnNo)}-${pad2(layerNo)}`;

  await page.getByRole("button", { name: "批量", exact: true }).click();
  await page.getByLabel("区域编码").fill(area);
  await page.getByLabel("排起始").fill(rowNo);
  await page.getByLabel("排结束").fill(rowNo);
  await page.getByLabel("列起始").fill(columnNo);
  await page.getByLabel("列结束").fill(columnNo);
  await page.getByLabel("层起始").fill(layerNo);
  await page.getByLabel("层结束").fill(layerNo);
  await page.getByRole("button", { name: "确认新增" }).click();

  await expect(page.getByText("已新增 1 个库位")).toBeVisible();
  await expect(page.getByText(createdCode).first()).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "locations-batch-create.png"),
    fullPage: false,
  });

  const docksMenu = page.getByRole("navigation").getByRole("button", { name: /M1 月台管理/ });
  const storageGroup = page.getByRole("navigation").getByRole("button", { name: "仓储资料", exact: true });
  if ((await storageGroup.getAttribute("aria-expanded")) !== "true") await storageGroup.click();
  await expect(docksMenu).toBeVisible();
  await docksMenu.click();
  await expect(page.getByRole("heading", { name: "M1 月台管理" })).toBeVisible();
  const dockCode = `D-E2E-${Date.now()}`;
  await page.getByRole("button", { name: "新增", exact: true }).click();
  const createDockDialog = page.getByRole("dialog");
  await createDockDialog.getByLabel("月台编号").fill(dockCode);
  await createDockDialog.getByLabel("作业类型").selectOption("receiving");
  await createDockDialog.getByLabel("温区").selectOption("cold_chain");
  await createDockDialog.getByLabel("位置说明").fill("E2E 冷链收货月台");
  await createDockDialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(createDockDialog).toBeHidden();
  await page.getByLabel("关键字").fill(dockCode);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const dockRow = page.locator("tr", { hasText: dockCode }).first();
  await expect(dockRow).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "docks-created.png"), fullPage: false });

  await dockRow.getByRole("checkbox").click();
  await page.getByRole("button", { name: "预约", exact: true }).click();
  const appointmentDialog = page.getByRole("dialog");
  const appointmentNo = `AP-E2E-${Date.now()}`;
  await appointmentDialog.getByLabel("预约编号").fill(appointmentNo);
  await appointmentDialog.getByLabel("关联单据号").fill(`ASN-E2E-${Date.now()}`);
  await appointmentDialog.getByLabel("车牌号").fill("沪A12345");
  await appointmentDialog.getByLabel("司机姓名").fill("E2E 司机");
  await appointmentDialog.getByLabel("司机电话").fill("13800000003");
  await appointmentDialog.getByRole("button", { name: "创建预约", exact: true }).click();
  await expect(appointmentDialog).toBeHidden();
  await expect(page.getByText(`预约 ${appointmentNo} 已创建`, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "dock-appointment-created.png"), fullPage: false });

  await page.getByRole("button", { name: "预约记录", exact: true }).click();
  const appointmentRecordsDialog = page.getByRole("dialog");
  await expect(appointmentRecordsDialog.getByText(appointmentNo, { exact: false })).toBeVisible();
  await appointmentRecordsDialog.getByRole("button", { name: "变更", exact: true }).click();
  const changeAppointmentDialog = page.getByRole("dialog").filter({ hasText: "变更月台预约" });
  await changeAppointmentDialog.getByLabel("预约开始").fill("2030-07-13T11:00");
  await changeAppointmentDialog.getByLabel("预约结束").fill("2030-07-13T12:00");
  await changeAppointmentDialog.getByLabel("变更原因").fill("E2E 调度变更");
  const changeResponse = page.waitForResponse((response) => response.url().includes("/api/v1/dock-appointments/") && response.request().method() === "PATCH");
  await changeAppointmentDialog.getByRole("button", { name: "保存变更", exact: true }).click();
  const changeResponseValue = await changeResponse;
  expect(changeResponseValue.status(), `PATCH ${changeResponseValue.url()}`).toBe(200);
  await expect(changeAppointmentDialog).toBeHidden();
  await expect(page.getByText(`预约 ${appointmentNo}-V2 已生成新版本`, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "dock-appointment-changed.png"), fullPage: false });

  const changedAppointmentRow = appointmentRecordsDialog.locator("tr", { hasText: `${appointmentNo}-V2` });
  await changedAppointmentRow.getByRole("button", { name: "取消", exact: true }).click();
  const cancelAppointmentDialog = page.getByRole("dialog").filter({ hasText: "取消月台预约" });
  await cancelAppointmentDialog.getByRole("button", { name: "确认取消", exact: true }).click();
  await expect(cancelAppointmentDialog).toBeHidden();
  await expect(page.getByText(`预约 ${appointmentNo}-V2 已取消`, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "dock-appointment-cancelled.png"), fullPage: false });
  await appointmentRecordsDialog.getByRole("button", { name: "关闭", exact: true }).click();
  await expect(appointmentRecordsDialog).toBeHidden();

  await page.getByRole("button", { name: "编辑", exact: true }).click();
  await page.getByRole("dialog").getByLabel("状态").selectOption("maintenance");
  await page.getByRole("dialog").getByLabel("预计恢复日期").fill("2026-07-20");
  await page.getByRole("dialog").getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeHidden();
  await expect(page.locator("tr", { hasText: dockCode }).getByText("维护中", { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "docks-maintenance.png"), fullPage: false });

  const importedDockCode = `D-E2E-IMPORT-${Date.now()}`;
  await page.locator('input[type="file"]').setInputFiles({
    name: "docks-e2e.csv",
    mimeType: "text/csv",
    buffer: Buffer.from(
      `月台编号,作业类型,温区,位置说明\n${importedDockCode},发货,常温,E2E 批量导入月台`,
      "utf8",
    ),
  });
  await expect(page.getByText("已导入 1 个月台", { exact: true })).toBeVisible();
  await page.getByLabel("关键字").fill(importedDockCode);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  await expect(page.locator("tr", { hasText: importedDockCode }).first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "docks-imported.png"), fullPage: false });

  const negativeResults = await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const headers = {
      Authorization: `Bearer ${session.accessToken}`,
      "Content-Type": "application/json",
    };
    const warehouses = await fetch("/api/v1/master-data/warehouses", { headers }).then((response) => response.json());
    const warehouse = warehouses.data[0];
    const zones = await fetch("/api/v1/master-data/warehouse-zones", { headers }).then((response) => response.json());
    const zone = zones.data[0];
    const invalidType = await fetch("/api/v1/master-data/warehouses", {
      method: "POST",
      headers: { ...headers, "Idempotency-Key": `m1-e2e-invalid-type-${Date.now()}` },
      body: JSON.stringify({ warehouse_code: `WH-INVALID-${Date.now()}`, warehouse_name: "非法类型", warehouse_type: "invalid" }),
    });
    const invalidOwner = await fetch("/api/v1/master-data/locations", {
      method: "POST",
      headers: { ...headers, "Idempotency-Key": `m1-e2e-invalid-owner-${Date.now()}` },
      body: JSON.stringify({
        warehouse_id: warehouse.id,
        zone_id: zone.id,
        location_code: "Z99-01-01-01",
        row_no: 1,
        column_no: 1,
        layer_no: 1,
        max_volume_cm3: 1000,
        max_sku_count: 1,
        location_type: "storage",
        bound_owner_id: "00000000-0000-0000-0000-000000009999",
      }),
    });
    return {
      invalidType: { status: invalidType.status, body: await invalidType.json() },
      invalidOwner: { status: invalidOwner.status, body: await invalidOwner.json() },
    };
  });
  expect(negativeResults.invalidType.status).toBe(422);
  expect(negativeResults.invalidType.body.code).toBe("M1_WAREHOUSE_TYPE_INVALID");
  expect(negativeResults.invalidOwner.status).toBe(422);
  expect(negativeResults.invalidOwner.body.code).toBe("M1_LOCATION_OWNER_INVALID");
  await page.screenshot({ path: path.join(artifactsDir, "negative-validation.png"), fullPage: false });
});

test("M1 供应商资质 PC 真实维护", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();
  const masterDataGroup = page.getByRole("navigation").getByRole("button", { name: "主数据", exact: true });
  if ((await masterDataGroup.getAttribute("aria-expanded")) !== "true") await masterDataGroup.click();
  await page.getByRole("navigation").getByRole("button", { name: /M1 客商档案/ }).click();
  await expect(page.getByRole("heading", { name: "M1 客商档案" })).toBeVisible();
  await expect(page.getByText("S-M1-E2E-001").first()).toBeVisible();

  const row = page.locator("tr", { hasText: "S-M1-E2E-001" }).first();
  await row.getByRole("button", { name: "编辑", exact: true }).click();
  const supplierDialog = page.getByRole("dialog");
  await supplierDialog.getByLabel("统一社会信用代码").fill("INVALID-USCC");
  await expect(supplierDialog.getByRole("button", { name: "保存", exact: true })).toBeDisabled();
  await page.screenshot({ path: path.join(artifactsDir, "supplier-qualification-invalid.png"), fullPage: false });

  await supplierDialog.getByLabel("统一社会信用代码").fill("91350100M000100Y43");
  await supplierDialog.getByLabel("联系人").fill("E2E 供应商联系人");
  const updateResponse = page.waitForResponse(
    (response) => response.url().includes("/api/v1/master-data/suppliers/") && response.request().method() === "PATCH",
  );
  await supplierDialog.getByRole("button", { name: "保存", exact: true }).click();
  const response = await updateResponse;
  expect(response.status(), await response.text()).toBe(200);
  await expect(supplierDialog).toBeHidden();
  await expect(page.getByText("E2E 供应商联系人").first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "supplier-qualification-updated.png"), fullPage: false });
});

test("M1 供应商批量导入调用原子批量接口", async ({ page }) => {
  fs.mkdirSync(businessPartnerEvidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();
  const masterDataGroup = page.getByRole("navigation").getByRole("button", { name: "主数据", exact: true });
  if ((await masterDataGroup.getAttribute("aria-expanded")) !== "true") await masterDataGroup.click();
  await page.getByRole("navigation").getByRole("button", { name: /M1 客商档案/ }).click();
  await expect(page.getByRole("heading", { name: "M1 客商档案" })).toBeVisible();

  const marker = Date.now();
  const firstCode = `S-E2E-BATCH-${marker}-1`;
  const secondCode = `S-E2E-BATCH-${marker}-2`;
  await page.getByRole("button", { name: "供入", exact: true }).click();
  await page.getByRole("textbox", { name: "批量导入供应商" }).fill([
    "supplier_code,supplier_name,license_no,contact_name",
    `${firstCode},E2E 批量供应商一,91310110666007217T,联系人一`,
    `${secondCode},E2E 批量供应商二,91110108MA01ABCD1E,联系人二`,
  ].join("\n"));
  const batchResponse = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/master-data/suppliers/batch-sync")
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "确认导入", exact: true }).click();
  const response = await batchResponse;
  expect(response.status(), await response.text()).toBe(200);
  expect(response.request().postDataJSON()).toHaveLength(2);
  await expect(page.getByText("已批量导入 2 个供应商", { exact: true })).toBeVisible();
  await expect(page.getByText(firstCode, { exact: true })).toBeVisible();
  await expect(page.getByText(secondCode, { exact: true })).toBeVisible();
  await page.screenshot({
    path: path.join(businessPartnerEvidenceDir, "supplier-batch-import-current.png"),
    fullPage: false,
  });
});

test("M1 客户批量导入调用原子批量接口", async ({ page }) => {
  fs.mkdirSync(businessPartnerEvidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();
  const masterDataGroup = page.getByRole("navigation").getByRole("button", { name: "主数据", exact: true });
  if ((await masterDataGroup.getAttribute("aria-expanded")) !== "true") await masterDataGroup.click();
  await page.getByRole("navigation").getByRole("button", { name: /M1 客商档案/ }).click();
  await expect(page.getByRole("heading", { name: "M1 客商档案" })).toBeVisible();

  const marker = Date.now();
  const firstCode = `C-E2E-BATCH-${marker}-1`;
  const secondCode = `C-E2E-BATCH-${marker}-2`;
  await page.getByRole("button", { name: "客入", exact: true }).click();
  await page.getByRole("textbox", { name: "批量导入客户" }).fill([
    "customer_code,customer_name,license_no",
    `${firstCode},E2E 批量客户一,LIC-E2E-1`,
    `${secondCode},E2E 批量客户二,`,
  ].join("\n"));
  const batchResponse = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/master-data/customers/batch-sync")
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "确认导入", exact: true }).click();
  const response = await batchResponse;
  expect(response.status(), await response.text()).toBe(200);
  expect(response.request().postDataJSON()).toHaveLength(2);
  await expect(page.getByText("已批量导入 2 个客户", { exact: true })).toBeVisible();
  await expect(page.getByText(firstCode, { exact: true })).toBeVisible();
  await expect(page.getByText(secondCode, { exact: true })).toBeVisible();
  await page.screenshot({
    path: path.join(businessPartnerEvidenceDir, "customer-batch-import-current.png"),
    fullPage: false,
  });
});

test("M-VR 双人策略矩阵真实保存", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();
  const systemGroup = page.getByRole("navigation").getByRole("button", { name: "系统配置", exact: true });
  if ((await systemGroup.getAttribute("aria-expanded")) !== "true") await systemGroup.click();
  await page.getByRole("navigation").getByRole("button", { name: /M1 系统字典/ }).click();
  await page.getByRole("button", { name: /特殊药品分类 special_drug_category/ }).click();
  await expect(page.getByText("双人作业矩阵", { exact: true })).toBeVisible();
  await page.getByLabel("矩阵确认人").selectOption("00000000-0000-0000-0000-000000000103");

  const policySelect = page.getByLabel(/普通药品 入库 收货 双人策略/);
  const response = page.waitForResponse(
    (value) => value.url().includes("/api/v1/m-vr/dual-person-policy/rules") && value.request().method() === "PUT",
  );
  await policySelect.selectOption("dual_scan");
  const responseValue = await response;
  expect(responseValue.status(), await responseValue.text()).toBe(200);
  await expect(policySelect).toHaveValue("dual_scan");
  fs.mkdirSync(systemDictionaryEvidenceDir, { recursive: true });
  await page.screenshot({ path: path.join(systemDictionaryEvidenceDir, "dual-person-policy-matrix.png"), fullPage: true });
});

function pad2(value: string) {
  return value.padStart(2, "0");
}
