import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("M-DI 药检平台配置使用真实 API 展示货主列表和校验弹窗", async ({ page }) => {
  await login(page);
  await openPage(page);
  await expect(page.getByRole("heading", { name: "M-DI 药检平台对接配置" })).toBeVisible();
  await page.getByRole("button", { name: "新增平台", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增药检平台" });
  await dialog.getByLabel("平台编码").fill(`e2e-di-${Date.now()}`);
  await dialog.getByLabel("平台名称").fill("E2E 药检平台");
  await dialog.getByLabel("API 地址").fill("not-a-url");
  await dialog.getByRole("button", { name: "保存配置" }).click();
  expect(await dialog.getByLabel("API 地址").evaluate((element) => !(element as HTMLInputElement).validity.valid)).toBe(true);
  await dialog.getByLabel("API 地址").fill("https://inspection.example.test/api");
  await dialog.getByLabel("API Key Vault 引用").fill("vault://wms/e2e/di/api-key");
  await dialog.getByRole("button", { name: "保存配置" }).click();
  await expect(page.getByText("药检平台配置已保存")).toBeVisible();
  await expect(page.getByText("API Key 已配置")).toBeVisible();
  await expect(page.locator("body")).not.toContainText("vault://wms/e2e/di/api-key");
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/m-di-platforms");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "platform-config.png"), fullPage: false });
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
  const target = navigation.getByRole("button", { name: /药检平台/ });
  const section = navigation.getByRole("button", { name: "入库业务", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: "入库作业", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await expect(target).toBeVisible();
  await target.click();
}
