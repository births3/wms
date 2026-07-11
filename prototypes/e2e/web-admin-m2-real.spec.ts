import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/m2-real/screenshots");
const supplierId = "00000000-0000-0000-0000-000000001101";
const warehouseId = "00000000-0000-0000-0000-000000001301";

test("M2 PC 真实入库链路落库并生成库存与审计", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  const receiptNo = `ASN-M2-E2E-${Date.now()}`;
  await login(page);

  await openMenu(page, "入库业务", "入库作业", /M2 收货管理/);
  await page.getByRole("button", { name: "新增", exact: true }).click();
  await page.getByLabel("ASN 号").fill(receiptNo);
  await page.getByLabel("单据类型", { exact: true }).selectOption("purchase_inbound");
  await page.getByLabel("供应商 ID").fill(supplierId);
  await page.getByLabel("仓库 ID").fill(warehouseId);
  await page.getByLabel("商品编码").fill("P-M1-E2E-001");
  await page.getByLabel("预报数量").fill("10");
  await page.getByRole("button", { name: "创建 ASN" }).click();
  await expect(page.getByText(`${receiptNo} 已创建`)).toBeVisible();

  await page.getByRole("button", { name: "放行", exact: true }).click();
  await expect(page.getByText(`${receiptNo} 已放行`)).toBeVisible();
  await page.getByRole("button", { name: "收货", exact: true }).click();
  await page.getByLabel("实际到货数量").fill("10");
  await page.getByLabel("缺货数量").fill("0");
  await page.getByLabel("拒收数量", { exact: true }).fill("0");
  await page.getByRole("button", { name: "确认收货" }).click();
  await expect(page.getByText(`${receiptNo} 收货已提交`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "receiving.png") });

  await openMenu(page, "入库业务", "入库作业", /M2 验收管理/);
  await expect(page.locator("table").getByText(receiptNo, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "验收", exact: true }).click();
  await page.getByLabel("验收批号").fill("B-M2-E2E-001");
  await page.getByLabel("通过数量").fill("10");
  await page.getByLabel("拒收数量", { exact: true }).fill("0");
  await page.getByLabel("生产日期").fill("2026-01-01");
  await page.getByLabel("有效期至").fill("2028-01-01");
  await page.getByLabel("追溯码").fill("TRACE-M2-E2E-001");
  await page.getByRole("dialog", { name: "验收" }).getByRole("combobox", { name: "质量状态" }).selectOption("qualified");
  for (const label of ["外观核对", "包装核对", "说明书核对", "标签核对"]) await page.getByLabel(label).fill("通过");
  await page.getByLabel("第一签字人").fill("11111111-1111-4111-8111-111111111111");
  await page.getByLabel("第二签字人 ID").fill("22222222-2222-4222-8222-222222222222");
  await page.getByRole("button", { name: "提交验收" }).click();
  await expect(page.getByText(`${receiptNo} 验收已提交`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inspection.png") });

  await openMenu(page, "入库业务", "入库作业", /M2 上架管理/);
  await expect(page.locator("table").getByText(receiptNo, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "上架", exact: true }).click();
  await page.getByLabel("上架商品编码").fill("P-M1-E2E-001");
  await page.getByLabel("上架批号").fill("B-M2-E2E-001");
  await page.getByLabel("数量", { exact: true }).fill("10");
  await page.getByLabel("实际库位").fill("A01-01-02-03");
  await page.getByRole("button", { name: "确认上架" }).click();
  await expect(page.getByText(`${receiptNo} 上架已提交`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "putaway.png") });

  await openMenu(page, "库内业务", "库存管理", /M3 批号管理/);
  await expect(page.getByText("B-M2-E2E-001").first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory.png") });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openMenu(page: import("@playwright/test").Page, section: string, group: string, item: RegExp) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: item });
  if (!(await target.isVisible())) {
    const sectionButton = navigation.getByRole("button", { name: section, exact: true });
    if ((await sectionButton.getAttribute("aria-expanded")) !== "true") await sectionButton.click();
    const groupButton = navigation.getByRole("button", { name: group, exact: true });
    if ((await groupButton.getAttribute("aria-expanded")) !== "true") await groupButton.click();
  }
  await target.click();
}
