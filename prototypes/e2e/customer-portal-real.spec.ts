import { expect, test, type APIRequestContext, type Page } from "@playwright/test";
import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const apiURL = process.env.PORTAL_E2E_API_URL ?? "http://127.0.0.1:19190";
const projectionKey = "portal-real-e2e-projection-key";
const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/customer-portal");
const storageRoot = process.env.PORTAL_H_FILE_STORAGE_ROOT ??
  path.join(evidenceDir, "storage");
const customerId = "00000000-0000-0000-0000-000000007001";
const addressA = "00000000-0000-0000-0000-000000007101";
const addressB = "00000000-0000-0000-0000-000000007102";
const ownerId = "00000000-0000-0000-0000-000000000001";
const runTag = `DI-${Date.now()}`;
const orderA = crypto.randomUUID();
const orderA2 = crypto.randomUUID();
const orderMissing = crypto.randomUUID();
const orderProcessing = crypto.randomUUID();
const orderOversize = crypto.randomUUID();
const productA = crypto.randomUUID();
const productMissing = crypto.randomUUID();
const productProcessing = crypto.randomUUID();
const productOversize = crypto.randomUUID();
const currentReportVersion = crypto.randomUUID();
const historicalReportVersion = crypto.randomUUID();
const reportA = crypto.randomUUID();

test.describe.serial("独立客户药检单平台真实链路", () => {
  test.beforeAll(async ({ request }) => {
    fs.mkdirSync(path.join(storageRoot, "wms-attachments"), { recursive: true });
    fs.mkdirSync(evidenceDir, { recursive: true });
    fs.writeFileSync(
      path.join(storageRoot, "wms-attachments", `${runTag}-current.pdf`),
      Buffer.from("%PDF-1.4\n真实客户药检单当前版本\n%%EOF"),
    );
    fs.writeFileSync(
      path.join(storageRoot, "wms-attachments", `${runTag}-history.pdf`),
      Buffer.from("%PDF-1.4\n真实客户药检单历史版本\n%%EOF"),
    );
    fs.writeFileSync(
      path.join(storageRoot, "wms-attachments", `${runTag}-oversize.pdf`),
      Buffer.from("%PDF-1.4\n元数据超限测试\n%%EOF"),
    );

    await projectOrder(request, orderA, `${runTag}-A-001`, addressA, "shipped", productA, "BATCH-A");
    await projectOrder(request, orderA2, `${runTag}-A-002`, addressA, "signed", productA, "BATCH-A");
    await projectOrder(
      request,
      orderMissing,
      `${runTag}-B-MISSING`,
      addressB,
      "shipped",
      productMissing,
      "BATCH-MISSING",
    );
    await projectOrder(
      request,
      orderProcessing,
      `${runTag}-B-PROCESSING`,
      addressB,
      "signed",
      productProcessing,
      "BATCH-PROCESSING",
    );
    await projectOrder(
      request,
      orderOversize,
      `${runTag}-A-OVERSIZE`,
      addressA,
      "shipped",
      productOversize,
      "BATCH-OVERSIZE",
    );
    await projectReport(request, {
      id: historicalReportVersion,
      reportId: reportA,
      productId: productA,
      version: 1,
      reportNo: `${runTag}-REPORT-V1`,
      status: "superseded",
      current: false,
      copyStatus: "available",
      storageKey: `wms-attachments/${runTag}-history.pdf`,
      fileName: `${runTag}-history.pdf`,
      size: 43,
      reason: "更正前版本",
    });
    await projectReport(request, {
      id: currentReportVersion,
      reportId: reportA,
      productId: productA,
      version: 2,
      reportNo: `${runTag}-REPORT-V2`,
      status: "confirmed",
      current: true,
      copyStatus: "available",
      storageKey: `wms-attachments/${runTag}-current.pdf`,
      fileName: `${runTag}-current.pdf`,
      size: 43,
      reason: "报告编号更正",
    });
    await projectReport(request, {
      id: crypto.randomUUID(),
      reportId: crypto.randomUUID(),
      productId: productProcessing,
      version: 1,
      reportNo: `${runTag}-PROCESSING`,
      status: "confirmed",
      current: true,
      copyStatus: "processing",
      storageKey: null,
      fileName: null,
      size: null,
      reason: null,
    });
    await projectReport(request, {
      id: crypto.randomUUID(),
      reportId: crypto.randomUUID(),
      productId: productOversize,
      version: 1,
      reportNo: `${runTag}-OVERSIZE`,
      status: "confirmed",
      current: true,
      copyStatus: "available",
      storageKey: `wms-attachments/${runTag}-oversize.pdf`,
      fileName: `${runTag}-oversize.pdf`,
      size: 2 * 1024 * 1024 * 1024 + 1,
      reason: null,
    });
  });

  test("多地址、订单批号、当前版本、单份下载和真实 ZIP", async ({ page }) => {
    await login(page, "portal-multi", "login-desktop.png");
    await searchRun(page);
    await expect(page.locator("tbody tr")).toHaveCount(5);
    await expect(page.getByRole("combobox", { name: "客户地址" }).locator("option")).toHaveCount(3);
    await page.getByRole("combobox", { name: "客户地址" }).selectOption(addressA);
    await expect(page.locator("tbody tr")).toHaveCount(3);
    await capture(page, "address-scope.png");
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.getByRole("navigation", { name: "客户平台导航" })).toBeVisible();
    await capture(page, "orders-mobile.png");
    await page.setViewportSize({ width: 1280, height: 720 });

    await page.getByTestId(`portal-order-${runTag}-A-001`).getByRole("button", { name: "查看资料" }).click();
    await expect(page.getByTestId("portal-batch-BATCH-A")).toBeVisible();
    await expect(page.getByText(`${runTag}-REPORT-V2`)).toBeVisible();
    await expect(page.getByText(`${runTag}-REPORT-V1`)).toHaveCount(0);
    const closeDetail = page.getByRole("button", { name: "关闭订单详情" });
    await expect(closeDetail).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(page.getByRole("button", { name: "下载 PDF" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(closeDetail).toBeFocused();
    await capture(page, "order-batch-current.png");
    const reportDownload = page.waitForEvent("download");
    await page.getByRole("button", { name: "下载 PDF" }).click();
    const reportFile = await reportDownload;
    const reportPath = path.join(evidenceDir, "downloaded-current.pdf");
    await reportFile.saveAs(reportPath);
    expect(fs.readFileSync(reportPath, "utf8")).toContain("真实客户药检单当前版本");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await page.getByRole("combobox", { name: "客户地址" }).selectOption("");
    for (const orderNo of [
      `${runTag}-A-001`,
      `${runTag}-A-002`,
      `${runTag}-B-MISSING`,
    ]) {
      await page.getByRole("checkbox", { name: `选择订单 ${orderNo}` }).check();
    }
    const createExportResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith("/api/v1/exports")
        && response.request().method() === "POST",
    );
    await page.getByTestId("portal-create-export").click();
    const createdExport = await (await createExportResponse).json() as { id: string };
    await expect(page.getByText("批量任务已创建，可在导出中心查看进度")).toBeVisible();
    await capture(page, "batch-task-created.png");
    await page.getByRole("button", { name: "导出中心" }).first().click();
    const completed = page.locator(`tr[data-export-id="${createdExport.id}"]`);
    await expect(completed).toBeVisible({ timeout: 20_000 });
    await expect(completed).toContainText("1 份");
    await expect(completed).toContainText("1 项");
    await capture(page, "export-completed.png");
    const zipDownload = page.waitForEvent("download");
    await completed.getByRole("button", { name: "下载 ZIP" }).click();
    const zip = await zipDownload;
    const zipPath = path.join(evidenceDir, "downloaded-export.zip");
    await zip.saveAs(zipPath);
    const names = execFileSync("unzip", ["-Z1", zipPath], { encoding: "utf8" });
    expect(names.split("\n").filter((name) => name.startsWith("reports/"))).toHaveLength(1);
    expect(names).toContain("药检单清单.csv");
    const manifest = execFileSync("unzip", ["-p", zipPath, "药检单清单.csv"], { encoding: "utf8" });
    expect(manifest).toContain(`${runTag}-B-MISSING`);
    expect(manifest).toContain("资料暂缺");
  });

  test("处理中、历史权限和无地址越权均由真实查询库控制", async ({ page }) => {
    await login(page, "portal-multi");
    await searchRun(page);
    await page.getByTestId(`portal-order-${runTag}-B-PROCESSING`).getByRole("button", { name: "查看资料" }).click();
    await expect(page.getByText(`${runTag}-PROCESSING`)).toBeVisible();
    await expect(page.getByRole("button", { name: "处理中" })).toBeDisabled();
    await capture(page, "processing-state.png");

    await relogin(page, "portal-history");
    await searchRun(page);
    await page.getByTestId(`portal-order-${runTag}-A-001`).getByRole("button", { name: "查看资料" }).click();
    await expect(page.getByText(`${runTag}-REPORT-V1`)).toBeVisible();
    await expect(page.getByText("历史版本", { exact: true })).toBeVisible();
    await expect(page.getByText("更正原因：更正前版本")).toBeVisible();
    await capture(page, "history-visible.png");

    await relogin(page, "portal-none");
    await searchRun(page);
    await expect(page.getByText("当前账号和筛选范围内没有可查询订单")).toBeVisible();
    const crossScope = await page.evaluate(async (id) => {
      const session = JSON.parse(sessionStorage.getItem("wms-customer-portal-session") ?? "null") as {
        access_token?: string;
      } | null;
      const response = await fetch(`/api/v1/orders/${id}`, {
        headers: { Authorization: `Bearer ${session?.access_token ?? ""}` },
      });
      return response.status;
    }, orderA);
    expect(crossScope).toBe(404);
    await capture(page, "no-address-no-data.png");
  });

  test("客户管理员可建多账号，2GB 超限在页面明确拒绝", async ({ page }) => {
    await login(page, "portal-admin");
    await page.getByRole("button", { name: "客户账号" }).click();
    await page.getByRole("button", { name: "新建账号" }).click();
    const account = `e2e-${Date.now()}`;
    await page.getByLabel("用户名").fill(account);
    await page.getByLabel("显示名称").fill("E2E 单地址账号");
    await page.getByLabel("初始密码").fill("PortalAccount1!");
    await page.getByText("A · 上海浦东一店").click();
    await page.getByRole("button", { name: "保存账号" }).click();
    await expect(page.getByText(account)).toBeVisible();
    await capture(page, "multi-account-created.png");
    const accountRow = page.locator("tbody tr").filter({ hasText: account });
    await accountRow.getByRole("button", { name: "开启历史" }).click();
    await expect(accountRow.getByText("可查看")).toBeVisible();
    await accountRow.getByRole("button", { name: "停用" }).click();
    await expect(accountRow.getByText("停用", { exact: true })).toBeVisible();
    const disabledLoginStatus = await page.evaluate(async (username) => {
      const response = await fetch("/api/v1/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password: "PortalAccount1!" }),
      });
      return response.status;
    }, account);
    expect(disabledLoginStatus).toBe(401);
    await capture(page, "multi-account-managed.png");

    await page.getByRole("button", { name: "订单与药检单" }).click();
    await searchRun(page);
    await page.getByRole("checkbox", { name: `选择订单 ${runTag}-A-OVERSIZE` }).check();
    await page.getByTestId("portal-create-export").click();
    await expect(page.getByRole("alert")).toContainText("不超过 2GB");
    await capture(page, "export-2gb-rejected.png");
  });
});

async function login(page: Page, username: string, screenshotName?: string) {
  await page.goto("/");
  if (screenshotName) {
    await capture(page, screenshotName);
  }
  await page.getByLabel("用户名").fill(username);
  await page.getByLabel("密码").fill("CorrectHorse1!");
  await page.getByTestId("portal-login").click();
  await expect(page.getByRole("heading", { name: "订单与药检单" })).toBeVisible();
}

async function capture(page: Page, name: string) {
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: path.join(evidenceDir, name), fullPage: true });
}

async function relogin(page: Page, username: string) {
  await page.evaluate(() => sessionStorage.clear());
  await login(page, username);
}

async function searchRun(page: Page) {
  await page.getByLabel("订单关键词").fill(runTag);
  await page.getByRole("button", { name: "查询", exact: true }).click();
  await expect(page.getByText(new RegExp(`共 \\d+ 个订单`))).toBeVisible();
}

async function projectOrder(
  request: APIRequestContext,
  id: string,
  orderNo: string,
  addressId: string,
  status: "shipped" | "signed",
  productId: string,
  batchNo: string,
) {
  const now = new Date(Date.now() + 2_000).toISOString();
  await project(request, "customer_order.snapshot", {
    customer: {
      id: customerId,
      customer_code: "PORTAL-E2E",
      customer_name: "E2E 连锁客户",
      updated_at: now,
    },
    address: {
      id: addressId,
      customer_id: customerId,
      address_code: addressId === addressA ? "A" : "B",
      address_name: addressId === addressA ? "上海浦东一店" : "上海闵行二店",
      address_snapshot: { address_name: addressId === addressA ? "上海浦东一店" : "上海闵行二店" },
      updated_at: now,
    },
    order: {
      id,
      customer_id: customerId,
      order_no: orderNo,
      status,
      delivery_address_id: addressId,
      address_snapshot: { address_name: addressId === addressA ? "上海浦东一店" : "上海闵行二店" },
      shipped_at: now,
      signed_at: status === "signed" ? now : null,
      updated_at: now,
      lines: [{
        id: crypto.randomUUID(),
        product_id: productId,
        product_code: `P-${batchNo}`,
        product_name: `E2E 药品 ${batchNo}`,
        batch_no: batchNo,
        quantity: 10,
      }],
    },
  });
}

async function projectReport(
  request: APIRequestContext,
  input: {
    id: string;
    reportId: string;
    productId: string;
    version: number;
    reportNo: string;
    status: "confirmed" | "superseded";
    current: boolean;
    copyStatus: "available" | "processing";
    storageKey: string | null;
    fileName: string | null;
    size: number | null;
    reason: string | null;
  },
) {
  const now = new Date(Date.now() + input.version * 3_000).toISOString();
  await project(request, "drug_inspection_report.upsert", {
    id: input.id,
    report_id: input.reportId,
    owner_id: ownerId,
    product_id: input.productId,
    batch_no: input.productId === productA
      ? "BATCH-A"
      : input.productId === productProcessing
        ? "BATCH-PROCESSING"
        : "BATCH-OVERSIZE",
    version_number: input.version,
    report_no: input.reportNo,
    status: input.status,
    is_current: input.current,
    modification_reason: input.reason,
    customer_copy_status: input.copyStatus,
    customer_copy_storage_key: input.storageKey,
    customer_copy_file_name: input.fileName,
    customer_copy_size: input.size,
    customer_copy_hash: input.storageKey ? crypto.createHash("sha256").update(input.storageKey).digest("hex") : null,
    digitally_signed_original: input.productId === productA,
    confirmed_at: now,
    updated_at: now,
  });
}

async function project(request: APIRequestContext, eventType: string, payload: unknown) {
  const response = await request.post(`${apiURL}/api/v1/internal/projections`, {
    headers: { "X-Projection-Key": projectionKey },
    data: {
      event_id: crypto.randomUUID(),
      event_type: eventType,
      occurred_at: new Date().toISOString(),
      payload,
    },
  });
  expect(response.ok(), `${eventType}: ${await response.text()}`).toBeTruthy();
}
