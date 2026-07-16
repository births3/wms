import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m3-batches");
const statusConfigArtifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m3-status-config");

test("M3 库存查询使用真实 API 传递组合筛选并展示结果", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  const dictionaryResponsePromise = page.waitForResponse((response) =>
    response.url().includes("/api/v1/system-dictionaries/inventory_quality_status/items") &&
    response.request().method() === "GET",
  );
  await openInventoryBatches(page);
  const dictionaryResponse = await dictionaryResponsePromise;
  const dictionaryBody = await dictionaryResponse.json() as {
    data: Array<{ item_code: string; item_name: string; enabled: boolean }>;
  };
  expect(dictionaryResponse.ok()).toBeTruthy();
  expect(dictionaryBody.data).toEqual(expect.arrayContaining([
    expect.objectContaining({ item_code: "qualified", item_name: "合格", enabled: true }),
    expect.objectContaining({ item_code: "quarantined", item_name: "隔离", enabled: true }),
  ]));

  await page.getByRole("button", { name: "展开", exact: true }).click();
  await page.locator('input[aria-label="商品编码"][placeholder="按商品编码模糊查询"]').fill("P-M1-E2E-001");
  await page.locator('input[aria-label="批号"][placeholder="按批号模糊查询"]').fill("B-M4-E2E-001");
  await page.locator('input[aria-label="库位"][placeholder="按库位编码模糊查询"]').fill("A01-01-02-03");

  const responsePromise = page.waitForResponse((response) => {
    if (!response.url().includes("/api/v1/inventory/batches") || response.request().method() !== "GET") return false;
    const url = new URL(response.url());
    return url.searchParams.get("product_code") === "P-M1-E2E-001" &&
      url.searchParams.get("batch_no") === "B-M4-E2E-001" &&
      url.searchParams.get("location_code") === "A01-01-02-03";
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const response = await responsePromise;
  const body = await response.json() as { data: Array<{ batch_no: string; owner_id: string }> };
  expect(response.ok()).toBeTruthy();
  expect(body.data).toEqual(expect.arrayContaining([
    expect.objectContaining({ batch_no: "B-M4-E2E-001" }),
  ]));
  await expect(page.getByRole("status")).toContainText("批号列表已查询");
  await expect(page.getByText("B-M4-E2E-001", { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory-query.png"), fullPage: false });

  await page.getByRole("button", { name: "导出", exact: true }).click();
  const exportDialog = page.getByRole("dialog", { name: "导出列表" });
  await expect(exportDialog).toContainText("当前筛选结果共 1 条");
  await expect(exportDialog.getByLabel("导出格式")).toContainText("xlsx");
  await page.screenshot({ path: path.join(artifactsDir, "inventory-export-dialog.png"), fullPage: false });
  const downloadPromise = page.waitForEvent("download");
  await exportDialog.getByRole("button", { name: "导出", exact: true }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toMatch(/^M3批号管理_\d{10}\.xlsx$/);
  const downloadPath = await download.path();
  if (!downloadPath) throw new Error("M3 库存 Excel 下载未生成本地文件");
  expect(fs.statSync(downloadPath).size).toBeGreaterThan(0);
  expect(fs.readFileSync(downloadPath).subarray(0, 2).toString()).toBe("PK");
  await page.screenshot({ path: path.join(artifactsDir, "inventory-exported.png"), fullPage: false });
});

test("M3 库存状态规则使用真实 API 保存货主覆盖", async ({ page }) => {
  fs.mkdirSync(statusConfigArtifactsDir, { recursive: true });
  await login(page);
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M3 状态规则/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "库内业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "库存管理", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M3 库存状态管理" })).toBeVisible();

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增库存状态转换规则" });
  await dialog.getByLabel("起始状态").selectOption("qualified");
  await dialog.getByLabel("目标状态").selectOption("quarantined");
  await dialog.getByLabel("原因/审批来源").fill("E2E 质量隔离审批");
  const responsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/inventory/status-transitions/qualified/quarantined")
      && response.request().method() === "PUT",
  );
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  const response = await responsePromise;
  expect(response.status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("qualified → quarantined 规则已保存");
  await expect(page.getByRole("row").filter({ hasText: "E2E 质量隔离审批" })).toBeVisible();
  await page.screenshot({ path: path.join(statusConfigArtifactsDir, "owner-transition-saved.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openInventoryBatches(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M3 批号管理/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "库内业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "库存管理", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M3 批号管理" })).toBeVisible();
}
