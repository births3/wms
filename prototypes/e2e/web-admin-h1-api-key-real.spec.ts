import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/h1-api-key");

test("H1 API Key 管理使用真实 API 完成创建、轮换、吊销和截图验证", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openApiKeyPage(page);
  await expect(page.getByRole("heading", { name: "H1 API Key 生命周期" })).toBeVisible();

  const caller = `E2E 外部系统 ${Date.now()}`;
  await page.getByRole("button", { name: "创建 Key", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "创建 API Key" });
  await createDialog.getByLabel("调用方名称").fill(caller);
  await createDialog.getByLabel("用途").fill("E2E API Key 生命周期");
  await createDialog.getByRole("button", { name: "确认创建" }).click();
  await expect(page.getByText(/创建成功。明文 secret 只展示一次/)).toBeVisible();
  await expect(page.getByText(caller)).toBeVisible();

  await page.locator("tbody tr").filter({ hasText: caller }).getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "轮换", exact: true }).click();
  const rotateDialog = page.getByRole("dialog", { name: "轮换 API Key" });
  await rotateDialog.getByLabel("旧 Key 宽限期（天）").fill("2");
  await rotateDialog.getByRole("button", { name: "确认轮换" }).click();
  await expect(page.getByText(/轮换成功。新 secret 只展示一次/)).toBeVisible();

  await page.locator("tbody tr").filter({ hasText: caller }).first().getByRole("checkbox", { name: "选择此行" }).check();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "吊销", exact: true }).click();
  await expect(page.getByText("API Key 已吊销；重复吊销保持幂等")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "api-key-lifecycle.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openApiKeyPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /H1 API Key 管理/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "基础能力", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "H1 权限租户", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
}
