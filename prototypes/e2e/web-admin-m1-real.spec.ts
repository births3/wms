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
    { menu: /M1 客商档案/, title: "M1 客商档案", text: "S-M1-E2E-001", shot: "business-partners-supplier.png" },
    { menu: /M1 客商档案/, title: "M1 客商档案", text: "C-M1-E2E-001", shot: "business-partners-customer.png" },
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

  await page.getByRole("button", { name: /M1 库位管理/ }).click();
  await expect(page.getByRole("heading", { name: "M1 库位管理" })).toBeVisible();

  const marker = Date.now();
  const area = `Z${String((marker % 90) + 10)}`;
  const rowNo = ((Math.floor(marker / 100) % 90) + 10).toString();
  const columnNo = ((Math.floor(marker / 10_000) % 90) + 10).toString();
  const layerNo = ((Math.floor(marker / 1_000_000) % 90) + 10).toString();
  const createdCode = `${area}-${pad2(rowNo)}-${pad2(columnNo)}-${pad2(layerNo)}`;

  await page.getByRole("button", { name: "批量新增库位" }).click();
  await page.getByLabel("区域编码").fill(area);
  await page.getByLabel("排起始").fill(rowNo);
  await page.getByLabel("排结束").fill(rowNo);
  await page.getByLabel("列起始").fill(columnNo);
  await page.getByLabel("列结束").fill(columnNo);
  await page.getByLabel("层起始").fill(layerNo);
  await page.getByLabel("层结束").fill(layerNo);
  await page.getByRole("button", { name: "确认新增" }).click();

  await expect(page.getByText("已新增 1 个库位")).toBeVisible();
  await expect(page.getByText(createdCode).first()).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "locations-batch-create.png"),
    fullPage: false,
  });
});

function pad2(value: string) {
  return value.padStart(2, "0");
}
