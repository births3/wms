import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m1-lpn-containers");
const putawayReceiptNo = "ASN-LPN-E2E-001";
const putawayLocation = "A01-01-02-04";

test("M1 容器管理真实登录后完成创建、查询、策略、上架绑定和混批拒绝", async ({ page }) => {
  test.setTimeout(90_000);
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openMenu(page, "基础档案", "仓储资料", /M1 容器管理/);

  await expect(page.getByRole("heading", { name: "M1 容器管理", level: 2 })).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByText("当前库位", { exact: true }).first()).toBeVisible();
  const policyTitle = page.getByText("类型策略（默认禁止混批/混品）");
  await policyTitle.scrollIntoViewIfNeeded();
  await expect(policyTitle).toBeVisible();
  await expect(page.getByRole("checkbox", { name: "托盘混批" })).not.toBeChecked();

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "创建容器" });
  await expect(createDialog).toBeVisible();
  const createResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/master-data/lpn-containers") &&
      response.request().method() === "POST",
  );
  await createDialog.getByRole("button", { name: "保存", exact: true }).click();
  const created = await createResponse;
  expect(created.ok()).toBeTruthy();
  const body = (await created.json()) as { lpn_code?: string };
  expect(body.lpn_code).toMatch(/^LPN-PL-/);
  const lpnCode = body.lpn_code ?? "";
  await expect(page.getByText(lpnCode, { exact: true })).toBeVisible();

  const keyword = page.getByRole("textbox", { name: "关键字" });
  await keyword.fill(lpnCode);
  const filtered = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/master-data/lpn-containers") &&
      response.url().includes(`keyword=${encodeURIComponent(lpnCode)}`) &&
      response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filtered).ok()).toBeTruthy();
  await expect(page.getByText(lpnCode, { exact: true })).toBeVisible();

  await keyword.fill("LPN-NOT-EXIST");
  const emptyList = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/master-data/lpn-containers") &&
      response.url().includes("keyword=LPN-NOT-EXIST") &&
      response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await emptyList).ok()).toBeTruthy();
  await expect(page.getByText("暂无容器")).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "lpn-containers.png"),
    fullPage: false,
  });

  await openMenu(page, "入库业务", "入库作业", /M2 上架管理/);
  await expect(page.locator("table").getByText(putawayReceiptNo, { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "上架", exact: true }).click();
  const putawayDialog = page.getByRole("dialog", { name: "上架" });
  await expect(putawayDialog).toBeVisible();

  await fillPutawayForm(putawayDialog, {
    lpn: "LPN-MISSING",
    productCode: "P-M1-E2E-001",
    batchNo: "B-LPN-E2E-001",
    qty: "2",
    locationCode: putawayLocation,
  });
  const missingPutaway = page.waitForResponse(
    (response) =>
      response.url().includes("/putaway") &&
      response.request().method() === "POST",
  );
  await putawayDialog.getByRole("button", { name: "确认上架" }).click();
  expect((await missingPutaway).status()).toBe(422);
  await expect(putawayDialog.getByRole("alert").first()).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "lpn-putaway-unknown.png"),
    fullPage: false,
  });

  await fillPutawayForm(putawayDialog, {
    lpn: lpnCode,
    productCode: "P-M1-E2E-001",
    batchNo: "B-LPN-E2E-001",
    qty: "2",
    locationCode: putawayLocation,
  });
  const okPutaway = page.waitForResponse(
    (response) =>
      response.url().includes("/putaway") &&
      response.request().method() === "POST",
  );
  await putawayDialog.getByRole("button", { name: "确认上架" }).click();
  expect((await okPutaway).ok()).toBeTruthy();
  await expect(page.getByText(`${putawayReceiptNo} 上架已提交`)).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "lpn-putaway.png"),
    fullPage: false,
  });

  const inventory = await readInventory(page, lpnCode);
  expect(inventory.status).toBe(200);
  const rows = inventoryRows(inventory.body);
  expect(rows.some((row) => row.container_lpn === lpnCode && row.batch_no === "B-LPN-E2E-001")).toBeTruthy();

  await page.getByRole("button", { name: "上架", exact: true }).click();
  await expect(putawayDialog).toBeVisible();
  await fillPutawayForm(putawayDialog, {
    lpn: lpnCode,
    productCode: "P-M1-E2E-001",
    batchNo: "B-LPN-E2E-002",
    qty: "2",
    locationCode: putawayLocation,
  });
  const mixDenied = page.waitForResponse(
    (response) =>
      response.url().includes("/putaway") &&
      response.request().method() === "POST",
  );
  await putawayDialog.getByRole("button", { name: "确认上架" }).click();
  expect((await mixDenied).status()).toBe(422);
  await expect(putawayDialog.getByRole("alert").first()).toBeVisible();
  await page.screenshot({
    path: path.join(artifactsDir, "lpn-putaway-mix-denied.png"),
    fullPage: false,
  });
  await putawayDialog.getByRole("button", { name: "取消" }).click();
  await expect(putawayDialog).toBeHidden();

  await openMenu(page, "基础档案", "仓储资料", /M1 容器管理/);
  await page.getByRole("textbox", { name: "关键字" }).fill(lpnCode);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  await expect(page.getByText(lpnCode, { exact: true })).toBeVisible();
  await expect(page.getByText("在用")).toBeVisible();
});

async function login(page: Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openMenu(page: Page, section: string, group: string, item: RegExp) {
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

async function fillPutawayForm(
  dialog: ReturnType<Page["getByRole"]>,
  values: { lpn: string; productCode: string; batchNo: string; qty: string; locationCode: string },
) {
  await dialog.getByLabel("容器 LPN").fill(values.lpn);
  await dialog.getByLabel("上架商品编码").fill(values.productCode);
  await dialog.getByLabel("上架批号").fill(values.batchNo);
  await dialog.getByLabel("数量", { exact: true }).fill(values.qty);
  await dialog.getByLabel("实际库位").fill(values.locationCode);
}

async function readInventory(page: Page, lpnCode: string) {
  return page.evaluate(async (code) => {
    const session = JSON.parse(window.localStorage.getItem("wms.web-admin.auth-session") ?? "null") as {
      accessToken?: string;
    } | null;
    const response = await fetch(`/api/v1/inventory/batches?q=${encodeURIComponent(code)}`, {
      headers: session?.accessToken ? { Authorization: `Bearer ${session.accessToken}` } : undefined,
    });
    return { status: response.status, body: await response.json() };
  }, lpnCode);
}

function inventoryRows(body: unknown): Array<{ container_lpn?: string | null; batch_no?: string }> {
  if (!body || typeof body !== "object") return [];
  const record = body as { data?: unknown };
  return Array.isArray(record.data) ? (record.data as Array<{ container_lpn?: string | null; batch_no?: string }>) : [];
}
