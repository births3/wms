import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/shell-dev/screenshots");

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

test("H1 菜单管理能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H1 权限租户" }).click();
  await page.getByRole("button", { name: /H1 菜单管理/ }).click();

  const h1Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H1 菜单管理" }) });
  await expect(h1Page.getByText("菜单树")).toBeVisible();
  await expect(h1Page.getByText("节点配置")).toBeVisible();
  await expect(h1Page.getByRole("button", { name: "发布" })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h1-menu-management.png"), fullPage: false });
});

test("H2 H3 基础能力能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H2 审计能力" }).click();
  await page.getByRole("button", { name: /H2 审计追踪/ }).click();

  const h2Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H2 审计追踪" }) });
  await expect(h2Page.getByText("GET /api/v1/audit/events")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h2-audit-trail.png"), fullPage: false });

  await page.getByRole("button", { name: "H3 契约能力" }).click();
  await page.getByRole("button", { name: /H3 OpenAPI/ }).click();
  const h3Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H3 OpenAPI 契约" }) });
  await expect(h3Page.getByText("GET /openapi.json")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h3-api-contract.png"), fullPage: false });
});

test("H5 快递对接能通过三层菜单打开", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H5 快递能力" }).click();
  await page.getByRole("button", { name: /H5 快递对接/ }).click();

  const h5Page = page.locator("section").filter({ has: page.getByRole("heading", { name: "H5 快递对接" }) });
  await expect(h5Page.getByRole("heading", { name: "快递商配置" })).toBeVisible();
  await expect(h5Page.getByRole("heading", { name: "快递选择规则" })).toBeVisible();
  await expect(h5Page.getByText("顺丰速运")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "h5-express.png"), fullPage: false });
});
