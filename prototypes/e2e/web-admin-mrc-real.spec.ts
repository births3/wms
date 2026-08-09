import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/mrc-reconciliation");

test("M-RC 真实差异查询、选择隔离和频率配置", async ({ page }) => {
  const browserErrors = collectBrowserErrors(page);
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page);
  await page.route("**/api/v1/inventory/batches**", async (route) => {
    await route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ code: "M3_UNAVAILABLE", message: "库存批次服务暂不可用" }),
    });
  });

  const responsePromise = page.waitForResponse((response) =>
    response.url().includes("/api/v1/reconciliation/items") &&
    response.request().method() === "GET",
  );
  await openPage(page);
  const response = await responsePromise;
  expect(response.status(), await response.text()).toBe(200);
  const body = await response.json() as {
    data: Array<{ product_code: string; difference_type: string; resolution_status: string }>;
  };
  expect(body.data).toEqual(expect.arrayContaining([
    expect.objectContaining({
      product_code: "P-M1-E2E-001",
      difference_type: "wms_more",
      resolution_status: "open",
    }),
    expect.objectContaining({
      product_code: "P-RC-E2E-ERP",
      difference_type: "erp_more",
      resolution_status: "open",
    }),
  ]));
  await expect(page.getByText("P-M1-E2E-001", { exact: true })).toBeVisible();
  await expect(page.getByText("P-RC-E2E-ERP", { exact: true })).toBeVisible();
  await expect(page.getByText(/待处理表示真实对账已发现差异/)).toBeVisible();
  await expectNoDocumentOverflow(page);
  await page.getByRole("heading", { name: "M-RC 库存对账" }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(screenshotDir, "difference-list.png"), fullPage: false });

  const erpRow = page.locator("tbody tr").filter({ hasText: "P-RC-E2E-ERP" });
  await erpRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "账面", exact: true }).click();
  const erpDialog = page.getByRole("dialog", { name: "以 ERP 账面为准" });
  await expect(erpDialog.getByText(/读取目标库存批次失败/)).toBeVisible();
  await expect(erpDialog.getByRole("button", { name: "确认处理", exact: true })).toBeDisabled();
  await erpDialog.getByRole("button", { name: "取消", exact: true }).click();
  await erpRow.getByRole("checkbox", { name: "选择此行" }).uncheck();
  await page.unroute("**/api/v1/inventory/batches**");

  const differenceType = page.locator('summary[aria-label="差异类型"]').locator("..");
  await differenceType.locator("summary").click();
  await differenceType.getByText("ERP 多", { exact: true }).click();
  await page.getByRole("heading", { name: "M-RC 库存对账" }).click();
  await expect(differenceType).not.toHaveAttribute("open", "");
  const filteredResponsePromise = page.waitForResponse((candidate) => {
    if (!candidate.url().includes("/api/v1/reconciliation/items") || candidate.request().method() !== "GET") return false;
    const url = new URL(candidate.url());
    return url.searchParams.get("difference_type") === "wms_more";
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filteredResponsePromise).status()).toBe(200);
  await expect(page.getByText("P-RC-E2E-ERP", { exact: true })).toHaveCount(0);

  const row = page.locator("tbody tr").filter({ hasText: "P-M1-E2E-001" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  const isolatePromise = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/api/v1/reconciliation/items/isolation") &&
    candidate.request().method() === "POST",
  );
  await page.getByRole("button", { name: "隔离", exact: true }).click();
  const isolateDialog = page.getByRole("dialog", { name: "确认对账隔离" });
  await isolateDialog.getByRole("button", { name: "确认隔离", exact: true }).click();
  expect((await isolatePromise).status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("隔离完成，共处理 1 个库存批次");

  await page.getByRole("button", { name: "归档", exact: true }).click();
  const resolveDialog = page.getByRole("dialog", { name: "归档为已知差异" });
  const resolvePromise = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/resolve") && candidate.request().method() === "POST",
  );
  await resolveDialog.getByRole("button", { name: "确认处理", exact: true }).click();
  expect((await resolvePromise).status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("已归档为已知差异，并释放已有对账隔离");
  await expectNoDocumentOverflow(page);
  await page.getByRole("heading", { name: "M-RC 库存对账" }).scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(screenshotDir, "actions-completed.png"), fullPage: false });

  await page.getByRole("button", { name: "对账频率", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "维护定时对账频率" });
  await dialog.getByLabel("对账间隔（小时）").fill("6");
  const rulePromise = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/api/v1/reconciliation/rule") &&
    candidate.request().method() === "PUT",
  );
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  expect((await rulePromise).status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("对账频率已保存");
  expect(browserErrors).toEqual([]);
});

test("M-RC 多批次显式分配、异步状态与稳定加载更多", async ({ page }) => {
  const browserErrors = collectBrowserErrors(page);
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page);
  await openPage(page);

  await page.getByRole("textbox", { name: "商品编码", exact: true }).fill("P-RC-E2E-MULTI");
  await selectAdditionalResolutionStatus(page, "等待库存调整");
  const filteredResponse = page.waitForResponse((candidate) => {
    if (!candidate.url().includes("/api/v1/reconciliation/items")) return false;
    const url = new URL(candidate.url());
    return url.searchParams.get("product_code") === "P-RC-E2E-MULTI"
      && url.searchParams.get("resolution_status") === "open,adjustment_pending";
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filteredResponse).status()).toBe(200);

  const row = page.locator("tbody tr").filter({ hasText: "P-RC-E2E-MULTI" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  const isolatePromise = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/api/v1/reconciliation/items/isolation")
    && candidate.request().method() === "POST",
  );
  await page.getByRole("button", { name: "隔离", exact: true }).click();
  await page
    .getByRole("dialog", { name: "确认对账隔离" })
    .getByRole("button", { name: "确认隔离", exact: true })
    .click();
  expect((await isolatePromise).status()).toBe(200);
  await expect(page.getByRole("status")).toContainText("隔离完成，共处理 2 个库存批次");

  await page.getByRole("button", { name: "账面", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "以 ERP 账面为准" });
  const quantities = dialog.getByRole("spinbutton");
  await expect(quantities).toHaveCount(2);
  await quantities.nth(0).fill("1");
  await quantities.nth(1).fill("2");
  const resolvePromise = page.waitForResponse((candidate) =>
    candidate.url().endsWith("/resolve") && candidate.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "确认处理", exact: true }).click();
  const resolveResponse = await resolvePromise;
  expect(resolveResponse.status(), await resolveResponse.text()).toBe(200);
  const requestBody = resolveResponse.request().postDataJSON() as {
    disposition: string;
    allocations: Array<{ inventory_batch_id: string; quantity: string }>;
  };
  expect(requestBody.disposition).toBe("erp_truth");
  expect(requestBody.allocations).toHaveLength(2);
  expect(new Set(requestBody.allocations.map((allocation) => allocation.inventory_batch_id)).size).toBe(2);
  expect(requestBody.allocations.reduce((sum, allocation) => sum + Number(allocation.quantity), 0)).toBe(3);
  const resolvedItem = await resolveResponse.json() as {
    resolution_status: string;
    stock_adjustment_order_ids: string[];
  };
  expect(resolvedItem.resolution_status).toBe("adjustment_pending");
  expect(resolvedItem.stock_adjustment_order_ids).toHaveLength(2);
  await expect(row.getByText("等待库存调整", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "释放", exact: true })).toBeDisabled();
  await page.screenshot({
    path: path.join(screenshotDir, "adjustment-pending.png"),
    fullPage: false,
  });

  await page.getByRole("button", { name: "重置", exact: true }).click();
  await page.getByRole("textbox", { name: "商品编码", exact: true }).fill("P-RC-E2E-EXCEPTION");
  await replaceResolutionStatus(page, "处理异常");
  const exceptionResponse = page.waitForResponse((candidate) => {
    if (!candidate.url().includes("/api/v1/reconciliation/items")) return false;
    const url = new URL(candidate.url());
    return url.searchParams.get("product_code") === "P-RC-E2E-EXCEPTION"
      && url.searchParams.get("resolution_status") === "exception";
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await exceptionResponse).status()).toBe(200);
  const exceptionRow = page.locator("tbody tr").filter({ hasText: "P-RC-E2E-EXCEPTION" });
  await expect(exceptionRow.getByText("处理异常", { exact: true })).toBeVisible();
  await page.screenshot({
    path: path.join(screenshotDir, "exception-state.png"),
    fullPage: false,
  });

  await page.getByRole("button", { name: "重置", exact: true }).click();
  await page.getByRole("textbox", { name: "商品编码", exact: true }).fill("P-RC-PAGE-");
  await replaceResolutionStatus(page, "已处理");
  const firstPagePromise = page.waitForResponse((candidate) => {
    if (!candidate.url().includes("/api/v1/reconciliation/items")) return false;
    const url = new URL(candidate.url());
    return url.searchParams.get("product_code") === "P-RC-PAGE-"
      && url.searchParams.get("resolution_status") === "resolved"
      && !url.searchParams.has("cursor");
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const firstPage = await firstPagePromise;
  const firstBody = await firstPage.json() as {
    data: Array<{ id: string }>;
    page: { count: number; next_cursor: string | null };
  };
  expect(firstBody.page.count).toBe(50);
  expect(firstBody.page.next_cursor).not.toBeNull();
  const nextPagePromise = page.waitForResponse((candidate) => {
    if (!candidate.url().includes("/api/v1/reconciliation/items")) return false;
    return new URL(candidate.url()).searchParams.has("cursor");
  });
  await page.getByRole("button", { name: "加载更多", exact: true }).click();
  const nextPage = await nextPagePromise;
  const nextBody = await nextPage.json() as {
    data: Array<{ id: string }>;
    page: { count: number; next_cursor: string | null };
  };
  expect(nextBody.page.count).toBe(1);
  expect(nextBody.page.next_cursor).toBeNull();
  expect(new Set([...firstBody.data, ...nextBody.data].map((item) => item.id)).size).toBe(51);
  await expect(page.getByRole("button", { name: "加载更多", exact: true })).toHaveCount(0);
  expect(browserErrors).toEqual([]);
});

test("M-RC 对账频率读取失败时禁止用默认值覆盖", async ({ page }) => {
  const browserErrors = collectBrowserErrors(page);
  await login(page);
  await page.route("**/api/v1/reconciliation/rule", async (route) => {
    if (route.request().method() !== "GET") {
      await route.continue();
      return;
    }
    await route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ code: "RC_RULE_UNAVAILABLE", message: "对账规则暂不可用" }),
    });
  });

  await openPage(page);
  await expect(page.getByRole("alert")).toContainText("读取对账频率失败");
  await expect(page.getByRole("button", { name: "对账频率", exact: true })).toBeDisabled();
  expect(browserErrors).toEqual([]);
});

test("M-RC 写操作失败保留弹窗并显示中文错误", async ({ page }) => {
  const browserErrors = collectBrowserErrors(page);
  await login(page);
  await page.route("**/api/v1/reconciliation/items/*/resolve", async (route) => {
    await route.fulfill({
      status: 500,
      contentType: "application/json",
      body: JSON.stringify({ code: "RC_INTERNAL", message: "对账处置暂不可用" }),
    });
  });

  await openPage(page);
  const row = page.locator("tbody tr").filter({ hasText: "P-RC-E2E-ERP" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "实物", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "以 WMS 实物为准" });
  await dialog.getByRole("button", { name: "确认处理", exact: true }).click();
  await expect(dialog.getByRole("alert")).toContainText("对账处置暂不可用");
  await expect(dialog).toBeVisible();
  expect(browserErrors).toEqual([]);
});

test("M-RC 只读权限隐藏选择和全部写操作", async ({ page }) => {
  const browserErrors = collectBrowserErrors(page);
  await page.route("**/api/v1/auth/me", async (route) => {
    const response = await route.fetch();
    const user = await response.json() as { permissions: string[] };
    user.permissions = user.permissions.filter(
      (permission) =>
        permission !== "rc.reconciliation.execute"
        && permission !== "rc.reconciliation.resolve",
    );
    await route.fulfill({ response, json: user });
  });

  await login(page);
  await openPage(page);
  await expect(page.getByText(/当前账号只读/)).toBeVisible();
  await expect(page.getByRole("button", { name: "对账频率", exact: true })).toHaveCount(0);
  for (const action of ["隔离", "释放", "实物", "账面", "归档"]) {
    await expect(page.getByRole("button", { name: action, exact: true })).toHaveCount(0);
  }
  await expect(page.getByRole("checkbox", { name: "选择此行" })).toHaveCount(0);
  expect(browserErrors).toEqual([]);
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M-RC 库存对账/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "库内业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "库存管理", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M-RC 库存对账" })).toBeVisible();
}

async function selectAdditionalResolutionStatus(
  page: import("@playwright/test").Page,
  label: string,
) {
  const field = page.locator('summary[aria-label="处理状态"]').locator("..");
  await field.locator("summary").click();
  await field.getByRole("checkbox", { name: label, exact: true }).check();
  await page.getByRole("heading", { name: "M-RC 库存对账" }).click();
}

async function replaceResolutionStatus(
  page: import("@playwright/test").Page,
  label: string,
) {
  const field = page.locator('summary[aria-label="处理状态"]').locator("..");
  await field.locator("summary").click();
  await field.getByRole("checkbox", { name: "待处理", exact: true }).uncheck();
  await field.getByRole("checkbox", { name: label, exact: true }).check();
  await page.getByRole("heading", { name: "M-RC 库存对账" }).click();
}

function collectBrowserErrors(page: import("@playwright/test").Page) {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (
      message.type() === "error"
      && !message.text().startsWith("Failed to load resource:")
    ) {
      errors.push(`console: ${message.text()}`);
    }
  });
  page.on("pageerror", (error) => errors.push(`pageerror: ${error.message}`));
  return errors;
}

async function expectNoDocumentOverflow(page: import("@playwright/test").Page) {
  const dimensions = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    clientWidth: document.documentElement.clientWidth,
  }));
  expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.clientWidth);
}
