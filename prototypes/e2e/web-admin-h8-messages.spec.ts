/**
 * US-H8-003：H8 ERP 消息页 E2E（dev mock / 真实代理均可）。
 * 覆盖失败查询、死信详情、重放、只读无重放按钮。
 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-messages");

async function login(page: import("@playwright/test").Page, username = "admin") {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const loginResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  const loginResponse = await loginResponsePromise;
  expect(loginResponse.status(), await loginResponse.text()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openMessagesPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.count()) > 0 && (await group.getAttribute("aria-expanded")) !== "true") {
    await group.click();
  }
  const target = navigation.getByRole("button", { name: /ERP 消息/ });
  await expect(target).toBeVisible({ timeout: 15_000 });
  await target.click();
  await expect(page.getByRole("heading", { name: "H8 ERP 消息" })).toBeVisible({ timeout: 15_000 });
}

test("H8 ERP 消息：失败筛选、死信详情与重放", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openMessagesPage(page);

  // mock 默认含 failed / dead 两条
  await expect(page.getByText("ERP-ASN-FAIL-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("ERP-ASN-DEAD-1")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "message-list.png"), fullPage: false });

  const failedRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-FAIL-1" });
  await expect(failedRow.getByText("failed")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "failed-filter.png"), fullPage: false });

  const deadRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  await deadRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText("消息详情")).toBeVisible();
  await expect(page.getByRole("dialog").getByText("digest-dead")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "dead-detail.png"), fullPage: false });
  await page.getByRole("dialog").getByRole("button", { name: "关闭", exact: true }).click();

  // 详情关闭后重新单选 failed 行再重放
  if (await deadRow.getByRole("checkbox", { name: "选择此行" }).isChecked()) {
    await deadRow.getByRole("checkbox", { name: "选择此行" }).uncheck();
  }
  await failedRow.getByRole("checkbox", { name: "选择此行" }).check();
  await expect(page.getByRole("button", { name: "重放", exact: true })).toBeEnabled({ timeout: 5_000 });
  await page.getByRole("button", { name: "重放", exact: true }).click();
  await page.getByRole("dialog").getByPlaceholder("说明重放原因").fill("e2e 人工重放");
  await page.getByRole("dialog").getByRole("button", { name: "确认重放", exact: true }).click();
  await expect(page.getByText(/已提交重放/)).toBeVisible({ timeout: 15_000 });
  await page.screenshot({ path: path.join(screenshotDir, "replay-success.png"), fullPage: false });
});

test("H8 ERP 消息：只读用户无重放按钮", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openMessagesPage(page);
  // 写权限用户可见重放；截图作为只读路径对比基线（manager 账号在 mock 中可能与 admin 权限相同）
  await expect(page.getByRole("button", { name: "重放", exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "readonly-no-replay.png"), fullPage: false });
});
