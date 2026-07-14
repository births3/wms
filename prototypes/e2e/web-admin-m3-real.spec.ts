import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/m3-real/screenshots");

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
