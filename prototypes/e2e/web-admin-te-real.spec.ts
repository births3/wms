import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("M-TE 任务类型配置使用真实 API 展示预置类型并保存自定义类型", async ({ page }) => {
  await login(page);
  await openTaskTypePage(page);
  await expect(page.getByRole("heading", { name: "M-TE 任务类型配置" })).toBeVisible();
  await expect(page.locator("tbody tr").first()).toBeVisible();
  const code = `e2e_${Date.now()}`;
  await page.getByRole("button", { name: "新增类型", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增任务类型" });
  await dialog.getByLabel("类型编码").fill(code);
  await dialog.getByLabel("类型名称").fill("E2E 自定义任务");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("E2E 自定义任务")).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog", { name: "停用任务类型" }).getByRole("button", { name: "确认", exact: true }).click();
  await expect(row).toContainText("停用");
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/m-te-task-types");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "task-type-config.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) { await page.goto("/"); await page.getByLabel("货主编码").fill("PY_OWNER"); await page.getByLabel("登录账号").fill("admin"); await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!"); await page.getByRole("button", { name: "登录" }).click(); await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible(); }
async function openTaskTypePage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M-TE 任务类型配置/ });
  const section = navigation.getByRole("button", { name: "库内业务", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: "库存管理", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await expect(target).toBeVisible();
  await target.click();
}
