import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("H8 ERP 连接独立菜单：列表与新建确认", async ({ page }) => {
  await login(page);
  await openPage(page);
  await expect(page.getByRole("heading", { name: "H8 ERP 连接" })).toBeVisible();
  await page.getByRole("button", { name: "新建连接", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建 ERP 连接" });
  const code = `e2e-h8-${Date.now()}`;
  await dialog.locator("label", { hasText: "连接编码" }).locator("input").fill(code);
  await dialog.locator("label", { hasText: "连接名称" }).locator("input").fill("E2E ERP 连接");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText(code)).toBeVisible();

  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-connectors");
  fs.mkdirSync(screenshotDir, { recursive: true });
  // 门禁登记产物名：connector-list.png
  await page.screenshot({ path: path.join(screenshotDir, "connector-list.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const target = navigation.getByRole("button", { name: /ERP 连接/ });
  await expect(target).toBeVisible();
  await target.click();
}
