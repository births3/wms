import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/h5-express");

test.describe("H5 快递对接真实链路", () => {
  test("US-H5-001 快递商配置写入真实 API、PostgreSQL 和审计", async ({ page }) => {
    await openH5(page);
    const code = unique("CARRIER");

    await page.getByRole("button", { name: "新增", exact: true }).nth(0).click();
    const dialog = page.getByRole("dialog", { name: "快递商配置" });
    await dialog.getByLabel("快递商编码").fill(code);
    await dialog.getByLabel("快递商名称").fill(`E2E 快递商 ${code}`);
    await dialog.getByLabel("接口地址").fill("https://carrier.e2e.invalid/api");
    await dialog.getByLabel("Key 别名").fill("e2e_key");
    await dialog.getByLabel("Secret 别名").fill("e2e_secret");
    await dialog.getByLabel("账号").fill("E2E-ACCOUNT");
    await dialog.getByLabel("条件参数 JSON").fill('{"region":["上海"]}');
    const responsePromise = page.waitForResponse(
      (response) => response.url().endsWith("/api/v1/express/carriers") && response.request().method() === "POST",
    );
    await dialog.getByRole("button", { name: "保存", exact: true }).click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    await expect(page.getByText(`E2E 快递商 ${code}`, { exact: true })).toBeVisible();
    await screenshot(page, "US-H5-001-carrier-saved.png");
  });

  test("US-H5-002 快递选择规则写入真实 API 并回读", async ({ page }) => {
    await openH5(page);
    const code = unique("RULE");

    await page.getByRole("button", { name: "新增", exact: true }).nth(1).click();
    const dialog = page.getByRole("dialog", { name: "快递选择规则" });
    await dialog.getByLabel("规则编码").fill(code);
    await dialog.getByLabel("规则名称").fill(`E2E 路由规则 ${code}`);
    await dialog.getByLabel("快递商编码").fill("E2E-CARRIER");
    await dialog.getByLabel("优先级").fill("20");
    await dialog.getByLabel("兜底策略").fill("manual_review");
    await dialog.getByLabel("匹配条件 JSON").fill('{"province":["上海","江苏"]}');
    const responsePromise = page.waitForResponse(
      (response) => response.url().endsWith("/api/v1/express/routing-rules") && response.request().method() === "POST",
    );
    await dialog.getByRole("button", { name: "保存", exact: true }).click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    await expect(page.getByText(`E2E 路由规则 ${code}`, { exact: true })).toBeVisible();
    await screenshot(page, "US-H5-002-rule-saved.png");
  });

  test("US-H5-003 快递面单预览保留真实运单数据并留下截图", async ({ page }) => {
    await openH5(page);
    const { waybillNo } = await createWaybill(page, "PRINT");

    await page.getByRole("button", { name: "打印面单", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "打印面单" });
    await expect(dialog).toContainText(waybillNo);
    await screenshot(page, "US-H5-003-waybill-print-preview.png");
    await dialog.getByRole("button", { name: "打印", exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(page.getByText(waybillNo, { exact: true })).toBeVisible();
  });

  test("US-H5-004 快递下单与取消在真实 PostgreSQL 中闭环", async ({ page }) => {
    await openH5(page);
    const { waybillNo } = await createWaybill(page, "CANCEL");
    await expect(page.getByText(waybillNo, { exact: true })).toBeVisible();
    await screenshot(page, "US-H5-004-waybill-pushed.png");

    await page.getByRole("button", { name: "取消", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "取消运单" });
    await expect(dialog).toContainText(waybillNo);
    const responsePromise = page.waitForResponse(
      (response) => response.url().includes(`/api/v1/express/waybills/${waybillNo}/cancel`) && response.request().method() === "POST",
    );
    await dialog.getByRole("button", { name: "确认取消", exact: true }).click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    const cancelled = await response.json() as { waybill_no?: string; status?: string };
    expect(cancelled).toMatchObject({ waybill_no: waybillNo, status: "cancelled" });
    await expect(dialog).toBeHidden();
    await expect(page.getByRole("button", { name: "取消", exact: true })).toBeDisabled();
    await expect(page.getByText("已取消", { exact: true })).toBeVisible();
    await screenshot(page, "US-H5-004-waybill-cancelled.png");
  });

  test("US-H5-005 轨迹查询回读真实缓存事件", async ({ page }) => {
    await openH5(page);
    const { waybillNo } = await createWaybill(page, "TRACK");

    await page.getByRole("button", { name: "轨迹", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "轨迹详情" });
    const responsePromise = page.waitForResponse(
      (response) => response.url().includes(`/api/v1/express/waybills/${waybillNo}/tracking`) && response.request().method() === "GET",
    );
    await dialog.getByRole("button", { name: "刷新轨迹", exact: true }).click();
    const response = await responsePromise;
    expect(response.status()).toBe(200);
    await expect(dialog).toContainText("快递下单成功，等待承运商揽收");
    await screenshot(page, "US-H5-005-tracking.png");
  });
});

async function openH5(page: Page) {
  await login(page);
  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H5 快递能力", exact: true }).click();
  await page.getByRole("button", { name: /H5 快递对接 platform\.h5\.express/ }).click();
  await expect(page.getByRole("heading", { name: "运单作业", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: "快递选择规则", exact: true })).toBeVisible();
}

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录", exact: true }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function createWaybill(page: Page, label: string) {
  const carrierCode = unique(`CARRIER-${label}`);
  const packageNo = unique(`PACKAGE-${label}`);
  await page.getByRole("button", { name: "新增", exact: true }).nth(0).click();
  const carrierDialog = page.getByRole("dialog", { name: "快递商配置" });
  await carrierDialog.getByLabel("快递商编码").fill(carrierCode);
  await carrierDialog.getByLabel("快递商名称").fill(`E2E 承运商 ${label}`);
  await carrierDialog.getByLabel("接口地址").fill("https://carrier.e2e.invalid/api");
  await carrierDialog.getByLabel("条件参数 JSON").fill("{}");
  const carrierResponse = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/express/carriers") && response.request().method() === "POST",
  );
  await carrierDialog.getByRole("button", { name: "保存", exact: true }).click();
  expect((await carrierResponse).status()).toBe(200);

  const row = page.locator("tbody tr").filter({ hasText: carrierCode }).first();
  await expect(row).toBeVisible();
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "下单", exact: true }).click();
  const waybillDialog = page.getByRole("dialog", { name: "快递下单" });
  await waybillDialog.getByLabel("包裹号").fill(packageNo);
  await waybillDialog.getByLabel("快递商编码").fill(carrierCode);
  await waybillDialog.getByLabel("件数").fill("1");
  await waybillDialog.getByLabel("重量 g").fill("1200");
  await waybillDialog.getByLabel("体积 cm3").fill("8000");
  await waybillDialog.getByLabel("寄件人").fill("E2E WMS 仓库");
  await waybillDialog.getByLabel("寄件电话").fill("13800000000");
  await waybillDialog.getByLabel("寄件地址").fill("上海市浦东新区 E2E 仓");
  await waybillDialog.getByLabel("收件人").fill("E2E 客户");
  await waybillDialog.getByLabel("收件电话").fill("13900000000");
  await waybillDialog.getByLabel("收件地址").fill("上海市黄浦区 E2E 门店");
  const waybillResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/express/waybills") && response.request().method() === "POST",
  );
  await waybillDialog.getByRole("button", { name: "下单", exact: true }).click();
  const waybillResponse = await waybillResponsePromise;
  expect(waybillResponse.status()).toBe(200);
  const created = await waybillResponse.json() as { waybill_no?: string };
  const waybillNo = created.waybill_no;
  if (!waybillNo) throw new Error("真实下单响应未返回 waybill_no");
  await expect(page.getByText(waybillNo, { exact: true })).toBeVisible();
  return { waybillNo };
}

async function screenshot(page: Page, file: string) {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await page.screenshot({ path: path.join(artifactsDir, file), fullPage: false });
}

function unique(prefix: string) {
  return `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`.toUpperCase();
}
