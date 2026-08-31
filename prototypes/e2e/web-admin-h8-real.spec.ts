import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-connectors");
// REST 连通性探查会真实 GET `<REST 地址>/healthz` 且受端点白名单约束（见 h8-real 配置），
// e2e 统一指向本地后端的 /api/v1（其 /api/v1/healthz 恒 200）。
const probeApiBase = `${process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19199"}/api/v1`;

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
  // AC7：测试阶段即校验与 active 路由重叠（不通过则不可启用）
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });
  await expect(rowB.getByText("失败")).toBeVisible({ timeout: 10_000 });
  await expect(rowB.getByText("testing")).toBeVisible();

  if (!(await rowB.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await rowB.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "启用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(/test|测试|overlap|重叠/i, {
    timeout: 20_000,
  });
  await expect(rowB.getByText("testing")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "route-overlap-rejected.png"), fullPage: false });

  // 清理：关闭弹层后停用 A，避免占用默认 inbound/asn 全仓路由
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  const rowA = page.locator("tbody tr").filter({ hasText: codeA });
  if (await rowA.getByRole("checkbox", { name: "选择此行" }).isChecked()) {
    await rowA.getByRole("checkbox", { name: "选择此行" }).uncheck();
  }
  if (await rowB.getByRole("checkbox", { name: "选择此行" }).isChecked()) {
    await rowB.getByRole("checkbox", { name: "选择此行" }).uncheck();
  }
  await rowA.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已停用")).toBeVisible({ timeout: 20_000 });
});

test("H8 ERP 连接：仓库主管只读（无写操作）", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  // 先用管理员造一条数据
  await login(page, "admin");
  // 种子缺口补授：warehouse_manager 角色缺少 h1.auth.me（工作台菜单权限键），否则后端按权限
  // 过滤后其菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”不渲染任何菜单，
  // 后续 wh-manager 登录将无法经菜单导航到 H8 ERP 连接页。
  await grantWhManagerMenu(page);
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

test("H8 ERP 连接：编辑端点后回 testing 并需复测", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openPage(page);
  // 出站连接不与默认 inbound/asn 全仓 active 冲突
  const code = `e2e-h8-edit-${Date.now()}`;
  await page.getByRole("button", { name: "新建连接", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "新建 ERP 连接" });
  await createDialog.locator("label", { hasText: "连接编码" }).locator("input").fill(code);
  await createDialog.locator("label", { hasText: "连接名称" }).locator("input").fill("待编辑连接");
  await createDialog.locator("label", { hasText: "方向" }).locator("select").selectOption("outbound");
  await createDialog.locator("label", { hasText: "REST 地址" }).locator("input").fill(probeApiBase);
  await createDialog
    .locator("label", { hasText: "Bearer secret alias" })
    .locator("input")
    .fill("vault://wms/e2e/h8/bearer");
  const createResponsePromise = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/config/erp-connectors") &&
      response.request().method() === "POST" &&
      !response.url().includes("/test"),
  );
  await createDialog.getByRole("button", { name: "保存", exact: true }).click();
  expect((await createResponsePromise).status()).toBe(201);
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });

  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "测试", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("测试已完成")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("通过")).toBeVisible();
  if (!(await row.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await row.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "启用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已启用")).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("active")).toBeVisible();

  if (!(await row.getByRole("checkbox", { name: "选择此行" }).isChecked())) {
    await row.getByRole("checkbox", { name: "选择此行" }).check();
  }
  await page.getByRole("button", { name: "编辑", exact: true }).click();
  const editDialog = page.getByRole("dialog", { name: "编辑 ERP 连接" });
  await editDialog.locator("label", { hasText: "REST 地址" }).locator("input").fill("https://erp-edited.example.com");
  const patchPromise = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/config/erp-connectors/") &&
      response.request().method() === "PATCH",
  );
  await editDialog.getByRole("button", { name: "保存", exact: true }).click();
  const patch = await patchPromise;
  expect(patch.status(), await patch.text()).toBe(200);
  await expect(page.getByText(/已保存/)).toBeVisible({ timeout: 20_000 });
  await expect(row.getByText("testing")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "connector-edited-retesting.png"), fullPage: false });
});

test("H8 ERP 连接：route-resolve 返回唯一 active 与最小 scope", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openPage(page);
  const code = `e2e-h8-route-${Date.now()}`;
  await createConnector(page, code, "路由解析连接");
  await expect(page.getByText("已创建（testing）")).toBeVisible({ timeout: 20_000 });
  await testAndActivate(page, code);

  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as {
      accessToken?: string;
    } | null;
    return session?.accessToken ?? "";
  });
  const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19199";
  const resolved = await page.request.get(
    `${apiURL}/api/v1/config/erp-connectors/route-resolve?direction=inbound&message_type=asn`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(resolved.status(), await resolved.text()).toBe(200);
  const body = (await resolved.json()) as {
    connector: { connector_code: string; status: string };
    required_inbound_scopes: string[];
  };
  expect(body.connector.status).toBe("active");
  expect(body.connector.connector_code).toBe(code);
  expect(body.required_inbound_scopes).toContain("inbound:push");
  await page.screenshot({ path: path.join(screenshotDir, "route-resolve.png"), fullPage: false });

  // 清理避免占路由
  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已停用")).toBeVisible({ timeout: 20_000 });
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
    .locator("label", { hasText: "API Key（入站鉴权）" })
    .locator("select")
    .selectOption("00000000-0000-0000-0000-000000000099");
  await dialog.locator("label", { hasText: "REST 地址" }).locator("input").fill(probeApiBase);
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

/** E2E 种子缺口补授：warehouse_manager 角色未授予 h1.auth.me（工作台菜单节点的权限键），
 *  后端按权限过滤后仓库主管的已发布菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”
 *  而不渲染任何菜单，导致 wh-manager 只读场景无法经菜单导航。此处用 admin 会话经 H1 角色
 *  权限 API 在 wh-manager 登录前补授该权限（幂等：已存在则跳过）。 */
async function grantWhManagerMenu(page: import("@playwright/test").Page) {
  await page.evaluate(async () => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null");
    const headers: Record<string, string> = {
      Authorization: `Bearer ${session.accessToken}`,
      "Content-Type": "application/json",
    };
    const rolesResponse = await fetch("/api/v1/auth/roles", { headers });
    if (!rolesResponse.ok) throw new Error(`列出角色失败: ${rolesResponse.status}`);
    const roles = (await rolesResponse.json()) as {
      items: Array<{ id: string; role_code: string; permission_codes: string[] }>;
    };
    const manager = roles.items.find((role) => role.role_code === "warehouse_manager");
    if (!manager) throw new Error("未找到 warehouse_manager 角色");
    if (manager.permission_codes.includes("h1.auth.me")) return;
    const response = await fetch(`/api/v1/auth/roles/${manager.id}/permissions`, {
      method: "PUT",
      headers: {
        ...headers,
        "Idempotency-Key": `e2e-wh-manager-menu-${Date.now()}`,
      },
      body: JSON.stringify({ permission_codes: [...manager.permission_codes, "h1.auth.me"] }),
    });
    if (!response.ok) {
      throw new Error(`补授 h1.auth.me 失败: ${response.status} ${await response.text()}`);
    }
  });
}

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
  // API Key 为必填下拉（选项异步加载），显式选中 e2e 种子 Key
  await dialog
    .locator("label", { hasText: "API Key（入站鉴权）" })
    .locator("select")
    .selectOption("00000000-0000-0000-0000-000000000099");
  await dialog.locator("label", { hasText: "REST 地址" }).locator("input").fill(probeApiBase);
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
