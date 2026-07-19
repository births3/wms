import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-connectors");

test("H8 ERP 连接：新建 → 测试 → 启用 → 停用（真实 API）", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page);
  await openPage(page);
  await expect(page.getByRole("heading", { name: "H8 ERP 连接" })).toBeVisible();
  await expect(page.getByText(/可维护/)).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "page-loaded.png"), fullPage: false });

  const code = `e2e-h8-${Date.now()}`;
  await createConnector(page, code, "E2E ERP 连接");
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText(code)).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "connector-list.png"), fullPage: false });

  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("通过")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "connector-tested.png"), fullPage: false });

  // 行可能在刷新后取消选择
  if (!(await row.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await row.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "启用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已启用")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("active")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "connector-activated.png"), fullPage: false });

  if (!(await row.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await row.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已停用")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("disabled")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "connector-disabled.png"), fullPage: false });
});

test("H8 ERP 连接：路由重叠时启用拒绝（真实 API）", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page);
  await openPage(page);

  const stamp = Date.now();
  const codeA = `e2e-h8-ov-a-${stamp}`;
  const codeB = `e2e-h8-ov-b-${stamp}`;
  await createConnector(page, codeA, "重叠连接 A");
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });
  await createConnector(page, codeB, "重叠连接 B");
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });

  await testAndActivate(page, codeA);
  await expect(page.locator("tbody tr").filter({ hasText: codeA }).getByText("active")).toBeVisible();

  const rowB = page.locator("tbody tr").filter({ hasText: codeB });
  await rowB.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });

  if (!(await rowB.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await rowB.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "启用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(/route overlap|重叠/i, { timeout: 20_000 });
  await expect(rowB.getByText("testing")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "route-overlap-rejected.png"), fullPage: false });
});

test("H8 ERP 连接：仓库主管只读（无写操作）", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  // 先用管理员造一条数据
  await login(page, "admin");
  await openPage(page);
  const code = `e2e-h8-ro-${Date.now()}`;
  await createConnector(page, code, "只读可见连接");
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });

  // 切换为仓库主管
  await page.getByRole("button", { name: /退出/ }).click();
  await login(page, "wh-manager");
  await openPage(page);
  await expect(page.getByRole("heading", { name: "H8 ERP 连接" })).toBeVisible();
  await expect(page.getByText(/集成中心 · US-H8-001 · .* · 只读 ·/)).toBeVisible();
  await expect(page.getByText(code)).toBeVisible();
  await expect(page.getByRole("button", { name: "新建连接", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "测试", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "启用", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "停用", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "删除", exact: true })).toHaveCount(0);

  // API 写操作 403
  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as {
      accessToken?: string;
    } | null;
    return session?.accessToken ?? "";
  });
  expect(token.length).toBeGreaterThan(10);
  const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19199";
  const denied = await page.request.post(`${apiURL}/api/v1/config/erp-connectors`, {
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      "Idempotency-Key": `e2e-ro-${Date.now()}`,
    },
    data: {
      connector_code: `deny-${Date.now()}`,
      connector_name: "should-deny",
      warehouse_ids: [],
      directions: ["inbound"],
      message_types: ["asn"],
      channel_mode: "rest",
      api_base_url: "https://erp.example.com",
      api_key_id: "00000000-0000-0000-0000-000000000099",
    },
  });
  expect(denied.status(), await denied.text()).toBe(403);
  await page.screenshot({ path: path.join(screenshotDir, "readonly-manager.png"), fullPage: false });
});

test("H8 ERP 连接：Vault alias 可解析后测试通过", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openPage(page);
  const code = `e2e-h8-vault-${Date.now()}`;
  await page.getByRole("button", { name: "新建连接", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建 ERP 连接" });
  await dialog.locator("label", { hasText: "连接编码" }).locator("input").fill(code);
  await dialog.locator("label", { hasText: "连接名称" }).locator("input").fill("Vault 出站连接");
  await dialog.locator("label", { hasText: "方向" }).locator("select").selectOption("outbound");
  await dialog
    .locator("label", { hasText: "Bearer secret alias" })
    .locator("input")
    .fill("vault://wms/e2e/h8/bearer");
  const createResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/config/erp-connectors") &&
      response.request().method() === "POST" &&
      !response.url().includes("/test"),
  );
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  const createResponse = await createResponsePromise;
  expect(createResponse.status(), await createResponse.text()).toBe(201);
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });

  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("通过")).toBeVisible();
  // 页面与审计不得回显 vault 明文 token
  await expect(page.locator("body")).not.toContainText("e2e-bearer-token");
  await page.screenshot({ path: path.join(screenshotDir, "vault-alias-resolved.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page, username = "admin") {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const loginResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  const loginResponse = await loginResponsePromise;
  expect(loginResponse.status(), await loginResponse.text()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  // 二级组可能需要展开
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.count()) > 0 && (await group.getAttribute("aria-expanded")) !== "true") {
    await group.click();
  }
  const target = navigation.getByRole("button", { name: /ERP 连接/ });
  await expect(target).toBeVisible({ timeout: 15_000 });
  await target.click();
}

async function createConnector(
  page: import("@playwright/test").Page,
  code: string,
  name: string,
) {
  await page.getByRole("button", { name: "新建连接", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建 ERP 连接" });
  await dialog.locator("label", { hasText: "连接编码" }).locator("input").fill(code);
  await dialog.locator("label", { hasText: "连接名称" }).locator("input").fill(name);
  const createResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/config/erp-connectors") &&
      response.request().method() === "POST" &&
      !response.url().includes("/test") &&
      !response.url().includes("/activate") &&
      !response.url().includes("/disable"),
  );
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  const createResponse = await createResponsePromise;
  expect(createResponse.status(), await createResponse.text()).toBe(201);
}

async function testAndActivate(page: import("@playwright/test").Page, code: string) {
  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });
  if (!(await row.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await row.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "启用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已启用")).toBeVisible({ timeout: 20_000 });
}
