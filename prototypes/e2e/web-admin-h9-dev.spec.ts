import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/h9-dev/screenshots");

test("H9 设计器使用浏览器真实全屏", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H9 打印能力" }).click();
  await page.getByRole("button", { name: /H9 打印模板/ }).click();
  await expect(page.getByRole("heading", { name: "H9 打印模板" })).toBeVisible();

  await page.getByRole("button", { name: "新增" }).click();
  const designer = page.locator('[data-h9-hiprint-designer="true"]');
  await expect(designer).toBeVisible();
  await expect(designer.getByText("模板编码")).toBeVisible();
  await expect(designer.getByText("模板与纸张设置")).toHaveCount(0);

  await page.getByRole("button", { name: "全屏" }).click();
  await expect.poll(() => page.evaluate(() => Boolean(document.fullscreenElement))).toBe(true);

  const box = await designer.boundingBox();
  const viewport = page.viewportSize();
  expect(box).not.toBeNull();
  expect(viewport).not.toBeNull();
  expect(Math.round(box?.x ?? -1)).toBe(0);
  expect(Math.round(box?.y ?? -1)).toBe(0);
  expect(Math.round(box?.width ?? 0)).toBe(viewport?.width);
  expect(Math.round(box?.height ?? 0)).toBe(viewport?.height);
  await page.screenshot({ path: path.join(artifactsDir, "designer-fullscreen.png"), fullPage: false });

  await page.getByRole("button", { name: "退出" }).click();
  await expect.poll(() => page.evaluate(() => Boolean(document.fullscreenElement))).toBe(false);
});
