/** US-H8-004：真实 PostgreSQL API + MSSQL DEMO 数据的浏览器与权限验收。 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-interface-tables");
const apiURL = process.env.WMS_WEB_ADMIN_H8_INTERFACE_REAL_API_URL ?? "http://127.0.0.1:18180";

test("H8 接口表探查：真实 DEMO 列表、筛选、详情且无写操作", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openInterfaceTables(page);

  await expect(page.getByText("DEMO-ASN-001", { exact: true })).toBeVisible();
  await expect(page.getByText("MSSQL 只读查询")).toBeVisible();
  await assertNoWriteActions(page);
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-list.png"), fullPage: false });

  await page.getByLabel("同步状态", { exact: true }).selectOption("pending");
  await page.getByRole("button", { name: "查询", exact: true }).click();
  await expect(page.getByText("DEMO-ASN-001", { exact: true })).toBeVisible();
  await expect(page.getByText(/合计 1/)).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-pending-filter.png"), fullPage: false });

  const row = page.locator("tbody tr").filter({ hasText: "DEMO-ASN-001" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByText("接口表行详情")).toBeVisible();
  await expect(dialog.getByText("报文摘要")).toBeVisible();
  await expect(dialog).not.toContainText("payload_json");
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-detail.png"), fullPage: false });
});

test("H8 接口表探查：仅有 connector.read 的新会话无菜单且 API 403", async ({ page }) => {
  await login(page, "wh-manager");
  await assertInterfaceMenuHidden(page);
  await page.reload();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
  await assertInterfaceMenuHidden(page);

  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as {
      accessToken?: string;
    } | null;
    return session?.accessToken ?? "";
  });
  expect(token.length).toBeGreaterThan(10);
  const denied = await page.request.get(`${apiURL}/api/v1/h8/erp-interface-tables/connectors`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  expect(denied.status(), await denied.text()).toBe(403);
  await page.screenshot({ path: path.join(screenshotDir, "interface-table-permission-denied.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page, username: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const responsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  const response = await responsePromise;
  expect(response.status(), await response.text()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openInterfaceTables(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  const target = navigation.getByRole("button", { name: /接口表探查/ });
  await expect(target).toBeVisible();
  await target.click();
  await expect(page.getByRole("heading", { name: "H8 接口表探查" })).toBeVisible();
}

async function assertInterfaceMenuHidden(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.count()) > 0 && (await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await expect(navigation.getByRole("button", { name: /接口表探查/ })).toHaveCount(0);
}

async function assertNoWriteActions(page: import("@playwright/test").Page) {
  for (const name of ["新增", "编辑", "删除", "保存", "重放"]) {
    await expect(page.getByRole("button", { name, exact: true })).toHaveCount(0);
  }
}
