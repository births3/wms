/** US-H8-004：受控接口表只读探查浏览器验收（dev mock 数据路径）。 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-interface-tables");

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const responsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  expect((await responsePromise).status()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openInterfaceTables(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.count()) > 0 && (await group.getAttribute("aria-expanded")) !== "true") await group.click();
  const target = navigation.getByRole("button", { name: /接口表探查/ });
  await expect(target).toBeVisible({ timeout: 15_000 });
  await target.click();
  await expect(page.getByRole("heading", { name: "H8 接口表探查" })).toBeVisible({ timeout: 15_000 });
}

test("H8 接口表探查：列表、状态筛选、详情且无写操作", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page);
  await openInterfaceTables(page);

  await expect(page.getByText("ASN-20260719-001")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("MSSQL 只读查询")).toBeVisible();
  await expect(page.getByRole("button", { name: "重放", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "新增", exact: true })).toHaveCount(0);
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-list.png"), fullPage: false });

  await page.getByLabel("同步状态", { exact: true }).selectOption("failed");
  await page.getByRole("button", { name: "查询", exact: true }).click();
  await expect(page.getByText("ASN-20260719-002")).toBeVisible();
  await expect(page.getByText("ASN-20260719-001")).toHaveCount(0);
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-failed-filter.png"), fullPage: false });

  const row = page.locator("tbody tr").filter({ hasText: "ASN-20260719-002" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText("接口表行详情")).toBeVisible();
  await expect(page.getByRole("dialog").getByText("报文摘要")).toBeVisible();
  await expect(page.getByRole("dialog")).not.toContainText("payload_json");
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-detail.png"), fullPage: false });
});
