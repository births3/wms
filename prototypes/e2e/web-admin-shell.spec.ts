import { expect, test } from "@playwright/test";

test("侧边栏筛选菜单支持 Escape 和点击页面内容关闭", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  const pageHeading = page.getByRole("heading", { name: "WMS Web Admin" });
  await expect(pageHeading).toBeVisible();

  await page.getByRole("button", { name: "筛选菜单" }).click();
  const menuFilter = page.getByLabel("筛选菜单");
  await expect(menuFilter).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(menuFilter).toBeHidden();

  await page.getByRole("button", { name: "筛选菜单" }).click();
  await expect(menuFilter).toBeVisible();

  await pageHeading.click();

  await expect(menuFilter).toBeHidden();
  await expect(page.getByRole("button", { name: "筛选菜单" })).toBeVisible();
});
