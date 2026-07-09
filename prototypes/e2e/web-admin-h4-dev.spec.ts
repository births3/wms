import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/h4-dev/screenshots");

test("H4 管理端菜单能打开通知配置和发送记录", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H4 企业微信" }).click();
  await page.getByRole("button", { name: /H4 参数设置/ }).click();
  await expect(page.getByRole("heading", { name: "H4 参数设置" })).toBeVisible();
  await expect(page.getByText("ww-demo-corp")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "wechat-settings.png"), fullPage: false });

  await page.getByRole("button", { name: /H4 通知配置/ }).click();
  await expect(page.getByRole("heading", { name: "H4 通知配置" })).toBeVisible();
  const configsPage = page.locator("section").filter({ has: page.getByRole("heading", { name: "H4 通知配置" }) });
  await expect(configsPage.getByText("asn_arrived").first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "notify-configs.png"), fullPage: false });

  await page.getByRole("button", { name: /H4 发送记录/ }).click();
  await expect(page.getByRole("heading", { name: "H4 发送记录" })).toBeVisible();
  const recordsPage = page.locator("section").filter({ has: page.getByRole("heading", { name: "H4 发送记录" }) });
  await expect(recordsPage.getByText("asn_arrived").first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "notify-records.png"), fullPage: false });
});
