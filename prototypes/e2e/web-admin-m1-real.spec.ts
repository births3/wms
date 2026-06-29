import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/m1-real/screenshots");

test("M1 管理端读取真实后端数据", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByLabel("密码").fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "WMS Web Admin" })).toBeVisible();

  await page.getByRole("button", { name: "基础档案" }).click();

  const cases = [
    { menu: /M1 商品档案/, title: "M1 商品档案", text: "P-M1-E2E-001", shot: "products.png" },
    { menu: /M1 供应商档案/, title: "M1 供应商档案", text: "S-M1-E2E-001", shot: "suppliers.png" },
    { menu: /M1 客户档案/, title: "M1 客户档案", text: "C-M1-E2E-001", shot: "customers.png" },
    { menu: /M1 仓库管理/, title: "M1 仓库管理", text: "WH-M1-E2E-001", shot: "warehouses.png" },
    { menu: /M1 库位管理/, title: "M1 库位管理", text: "A01-01-02-03", shot: "locations.png" },
    { menu: /M1 系统字典/, title: "M1 系统字典", text: "purchase_inbound", shot: "dictionary.png" },
  ];

  for (const item of cases) {
    await page.getByRole("button", { name: item.menu }).click();
    await expect(page.getByRole("heading", { name: item.title })).toBeVisible();
    await expect(page.getByText(item.text).first()).toBeVisible();
    await page.screenshot({ path: path.join(artifactsDir, item.shot), fullPage: false });
  }
});
