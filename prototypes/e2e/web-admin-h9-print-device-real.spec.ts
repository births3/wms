import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-print-devices");

test.use({ viewport: { width: 1600, height: 900 } });

test("US-H9-011 站点、打印机、纸盒、测试打印与租约人工释放", async ({ page }) => {
  fs.mkdirSync(evidenceDir, { recursive: true });
  const stamp = Date.now();
  const siteCode = `SITE-E2E-${stamp}`;
  const printerName = `E2E 验收打印机 ${stamp}`;
  await login(page);
  await openPrintDevices(page);

  // 站点页签：种子站点可见，新建站点并映射货主仓
  const seededSite = page.getByRole("row").filter({ hasText: "SITE-H9-E2E" });
  await expect(seededSite).toContainText("E2E 一号打印站");
  await page.getByRole("button", { name: "新建站点", exact: true }).click();
  const siteDialog = page.getByRole("dialog", { name: "新建物理打印站点" });
  await siteDialog.getByLabel("站点编码").fill(siteCode);
  await siteDialog.getByLabel("站点名称").fill("E2E 二号打印站");
  const createSiteResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-devices/sites")
      && response.request().method() === "POST",
  );
  await siteDialog.getByRole("button", { name: "创建站点" }).click();
  const createdSite = await createSiteResponse;
  expect(createdSite.ok(), await createdSite.text()).toBeTruthy();
  const siteRow = page.getByRole("row").filter({ hasText: siteCode });
  await expect(siteRow).toContainText("E2E 二号打印站");

  await siteRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "映射货主仓", exact: true }).click();
  const mappingDialog = page.getByRole("dialog", { name: "映射货主仓" });
  await expect(mappingDialog.getByText("当前货主")).toBeVisible();
  const mappingResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-devices/sites/")
      && response.url().endsWith("/owner-mappings")
      && response.request().method() === "POST",
  );
  await mappingDialog.getByRole("button", { name: "确认映射" }).click();
  expect((await mappingResponse).ok()).toBeTruthy();
  await expect(page.getByRole("row").filter({ hasText: "生效" }).first()).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "sites-and-mappings.png"), fullPage: false });

  // 打印机页签：新建打印机（归属新站点）并做释放模式单机覆盖
  await page.getByRole("tab", { name: /打印机/ }).click();
  const seededPrinter = page.getByRole("row").filter({ hasText: "E2E 东区网络打印机" });
  await expect(seededPrinter).toContainText("仅人工释放（全局默认）");
  await page.getByRole("button", { name: "新建打印机", exact: true }).click();
  const printerDialog = page.getByRole("dialog", { name: "新建打印机" });
  await printerDialog.getByLabel("所属站点").selectOption({ label: `${siteCode} E2E 二号打印站` });
  await printerDialog.getByLabel("打印机名称").fill(printerName);
  await printerDialog.getByLabel("型号（可选）").fill("Zebra ZT230");
  const createPrinterResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-devices/printers")
      && response.request().method() === "POST",
  );
  await printerDialog.getByRole("button", { name: "创建打印机" }).click();
  const createdPrinter = await createPrinterResponse;
  expect(createdPrinter.ok(), await createdPrinter.text()).toBeTruthy();
  const printerRow = page.getByRole("row").filter({ hasText: printerName });
  await expect(printerRow).toContainText("仅人工释放（全局默认）");

  await printerRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "释放模式覆盖", exact: true }).click();
  const releaseModeDialog = page.getByRole("dialog", { name: "释放模式覆盖" });
  await releaseModeDialog.getByLabel("覆盖模式").selectOption("safe_auto");
  const overrideResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-devices/printers/")
      && response.request().method() === "PATCH",
  );
  await releaseModeDialog.getByRole("button", { name: "保存覆盖" }).click();
  expect((await overrideResponse).ok()).toBeTruthy();
  await expect(printerRow).toContainText("安全自动释放（单机覆盖）");
  await page.screenshot({ path: path.join(evidenceDir, "printers.png"), fullPage: false });

  // 纸盒页签：为新打印机维护纸盒能力
  await page.getByRole("tab", { name: /纸盒/ }).click();
  const trayPanel = page.locator('[role="tabpanel"][data-state="active"]');
  await trayPanel.getByRole("combobox", { name: "打印机" }).selectOption({
    label: `${siteCode} · ${printerName}`,
  });
  await page.getByRole("button", { name: "新建纸盒", exact: true }).click();
  const trayDialog = page.getByRole("dialog", { name: "新建纸盒" });
  await trayDialog.getByLabel("纸盒设备标识").fill("TRAY-A");
  await trayDialog.getByLabel("纸张尺寸").fill("A5");
  await trayDialog.getByLabel("纸张类型").fill("不干胶标签纸");
  const createTrayResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/trays")
      && response.request().method() === "POST",
  );
  await trayDialog.getByRole("button", { name: "创建纸盒" }).click();
  expect((await createTrayResponse).ok()).toBeTruthy();
  const trayRow = page.getByRole("row").filter({ hasText: "TRAY-A" });
  await expect(trayRow).toContainText("A5");
  await expect(trayRow).toContainText("不干胶标签纸");
  await page.screenshot({ path: path.join(evidenceDir, "trays.png"), fullPage: false });

  // 测试打印：对指定打印机 + 纸盒下发受控测试指令并落表（真实硬件回执待 S4）
  await page.getByRole("tab", { name: /打印机/ }).click();
  await printerRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试打印", exact: true }).click();
  const testPrintDialog = page.getByRole("dialog", { name: "测试打印" });
  await expect(testPrintDialog.getByText("真实硬件回执")).toBeVisible();
  const testPrintResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/test-print")
      && response.request().method() === "POST",
  );
  await testPrintDialog.getByRole("button", { name: "下发测试打印" }).click();
  const testPrinted = await testPrintResponse;
  expect(testPrinted.ok(), await testPrinted.text()).toBeTruthy();
  const testPrintBody = await testPrinted.json() as { result: string };
  expect(testPrintBody.result).toBe("dispatched");
  await expect(page.getByRole("status")).toContainText("测试打印已下发");
  await page.screenshot({ path: path.join(evidenceDir, "test-print.png"), fullPage: false });

  // 租约页签：查看种子活动租约并授权人工释放（原因 + 二次确认）
  await page.getByRole("tab", { name: /租约/ }).click();
  const leaseRow = page.getByRole("row").filter({ hasText: "LEASE-H9-E2E-001" });
  await expect(leaseRow).toContainText("E2E 东区网络打印机");
  await expect(leaseRow).toContainText("仅人工释放");
  await expect(leaseRow).toContainText("空闲");
  await expect(leaseRow).toContainText("活动");
  await leaseRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "人工释放租约", exact: true }).click();
  const releaseDialog = page.getByRole("dialog", { name: "人工释放设备租约" });
  await expect(releaseDialog.getByText(/不能覆盖打印中/)).toBeVisible();
  const releaseButton = releaseDialog.getByRole("button", { name: "确认释放" });
  await releaseDialog.getByLabel("释放原因").fill("E2E 验收：打印机迁移，人工回收租约");
  await expect(releaseButton).toBeDisabled();
  await releaseDialog.getByRole("checkbox").check();
  const releaseResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-devices/leases/")
      && response.url().endsWith("/release")
      && response.request().method() === "POST",
  );
  await releaseButton.click();
  const released = await releaseResponse;
  expect(released.ok(), await released.text()).toBeTruthy();
  await expect(leaseRow).toContainText("已释放");
  await expect(leaseRow).toContainText("E2E 验收：打印机迁移，人工回收租约");
  await page.screenshot({ path: path.join(evidenceDir, "leases-released.png"), fullPage: false });
});

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openPrintDevices(page: Page) {
  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H9 打印能力", exact: true }).click();
  await page.getByRole("button", { name: /设备·Print Agent 管理/ }).click();
  await expect(page.getByRole("heading", { name: "设备·Print Agent 管理" })).toBeVisible();
}
