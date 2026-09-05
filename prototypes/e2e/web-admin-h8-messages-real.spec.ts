/** US-H8-003：关闭 dev-mock 的真实 PostgreSQL 浏览器验收。 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-messages");
const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19199";
const connectorId = "00000000-0000-0000-0000-000000008801";
const warehouseId = "00000000-0000-0000-0000-000000001301";

test("H8 ERP 消息真实链路：高级查询、重放、Worker 控制与加密报文", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  // 种子缺口补授：warehouse_manager 角色缺少 h1.auth.me（工作台菜单权限键），否则后端按权限
  // 过滤后其菜单树缺工作台，App 判定“已发布菜单为空或缺少工作台入口”不渲染任何菜单，
  // 本文件第二个测试的 wh-manager 只读场景将无法经菜单导航到 H8 ERP 消息页。
  await grantWhManagerMenu(page);
  const token = await accessToken(page);
  await openMessagesPage(page);

  const firstPage = await page.request.get(
    `${apiURL}/api/v1/integration/erp-messages?limit=1`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(firstPage.status(), await firstPage.text()).toBe(200);
  const firstPageBody = (await firstPage.json()) as {
    data: Array<{ id: string }>;
    page: { next_cursor: string | null };
  };
  expect(firstPageBody.data).toHaveLength(1);
  expect(firstPageBody.page.next_cursor).not.toBeNull();
  const secondPage = await page.request.get(
    `${apiURL}/api/v1/integration/erp-messages?limit=1&cursor=${encodeURIComponent(firstPageBody.page.next_cursor ?? "")}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(secondPage.status(), await secondPage.text()).toBe(200);
  const secondPageBody = (await secondPage.json()) as { data: Array<{ id: string }> };
  expect(secondPageBody.data).toHaveLength(1);
  expect(secondPageBody.data[0]?.id).not.toBe(firstPageBody.data[0]?.id);

  await expect(page.getByText("H8-MSG-E2E-DEAD", { exact: true })).toBeVisible();
  await expect(page.getByText("H8-MSG-E2E-FAIL", { exact: true })).toBeVisible();
  await expect(page.getByText("H8-MSG-E2E-OTHER-OWNER", { exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: "展开", exact: true }).click();
  await page.locator('summary[aria-label="消息类型"]').click();
  await page.getByRole("checkbox", { name: "预到货通知（ASN）", exact: true }).check();
  await page.locator('summary[aria-label="通道"]').click();
  await page.getByRole("checkbox", { name: "接口表", exact: true }).check();
  await page.locator('summary[aria-label="状态"]').click();
  await page.getByRole("checkbox", { name: "死信", exact: true }).check();
  await page.getByLabel("连接编码", { exact: true }).fill("H8-IF-E2E");
  await page.locator('summary[aria-label="仓库"]').click();
  await page.getByRole("checkbox", { name: warehouseId, exact: true }).check();
  await page.getByLabel("外部业务标识", { exact: true }).fill("H8-MSG-E2E-DEAD");
  await page.getByLabel("幂等键（Idempotency-Key）", { exact: true }).fill("h8-msg-e2e-dead-idem");
  await page.getByLabel("关联标识（Correlation）", { exact: true }).fill("h8-msg-e2e-dead-corr");
  const filtered = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.pathname === "/api/v1/integration/erp-messages" &&
      url.searchParams.get("message_type") === "asn" &&
      url.searchParams.get("channel") === "interface_table" &&
      url.searchParams.get("status") === "dead" &&
      url.searchParams.get("connector_code") === "H8-IF-E2E" &&
      url.searchParams.get("warehouse_id") === warehouseId &&
      url.searchParams.get("external_ref") === "H8-MSG-E2E-DEAD" &&
      url.searchParams.get("idempotency_key") === "h8-msg-e2e-dead-idem" &&
      url.searchParams.get("correlation_id") === "h8-msg-e2e-dead-corr"
    );
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filtered).status()).toBe(200);
  await expect(page.getByText("H8-MSG-E2E-DEAD", { exact: true })).toBeVisible();
  await expect(page.getByText("H8-MSG-E2E-FAIL", { exact: true })).toHaveCount(0);
  await page.screenshot({ path: path.join(screenshotDir, "message-list-real.png"), fullPage: false });

  const deadRow = page.locator("tbody tr").filter({ hasText: "H8-MSG-E2E-DEAD" });
  await deadRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText("h8-msg-e2e-dead-digest")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "dead-detail-real.png"), fullPage: false });
  await page.getByRole("dialog").getByRole("button", { name: "关闭", exact: true }).click();

  await page.getByRole("button", { name: "重放", exact: true }).click();
  await page.getByPlaceholder("说明重放原因").fill("真实 E2E 故障恢复");
  const replay = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000008901/replay") &&
      response.request().method() === "POST",
  );
  await page.getByRole("dialog").getByRole("button", { name: "确认重放", exact: true }).click();
  expect((await replay).status()).toBe(200);
  await expect(page.getByText(/已提交重放/)).toBeVisible();
  const duplicateReplay = await page.request.post(
    `${apiURL}/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000008901/replay`,
    {
      headers: { Authorization: `Bearer ${token}` },
      data: { reason: "重复重放不得复制消息", confirmed: true },
    },
  );
  expect(duplicateReplay.status(), await duplicateReplay.text()).toBe(409);
  const replayedMessage = await page.request.get(
    `${apiURL}/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000008901`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(replayedMessage.status(), await replayedMessage.text()).toBe(200);
  expect((await replayedMessage.json()).message.id).toBe(
    "00000000-0000-0000-0000-000000008901",
  );

  await page.getByRole("tab", { name: "Worker 状态" }).click();
  const inbound = page.locator("tbody tr").filter({ hasText: "入站" });
  await expect(inbound.getByText("h8-worker-real-e2e")).toBeVisible();
  await expect(inbound.getByText("健康", { exact: true })).toBeVisible();
  await inbound.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "暂停认领", exact: true }).click();
  await page.getByPlaceholder("说明操作原因").fill("真实 E2E 维护窗口");
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(inbound.getByText("已暂停", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "恢复认领", exact: true }).click();
  await page.getByPlaceholder("说明操作原因").fill("真实 E2E 维护完成");
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(inbound.getByText("运行中", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "报文保留", exact: true }).click();
  await page.getByLabel("启用完整报文加密保留").check();
  await page.getByLabel("保留天数（1–30）").fill("7");
  await page.getByRole("dialog").getByRole("button", { name: "确认保存", exact: true }).click();
  await expect(page.getByText("已启用完整报文保留（7 天）", { exact: true })).toBeVisible();

  const lifecycle = await page.request.post(`${apiURL}/api/v1/integration/erp-messages/lifecycle`, {
    headers: { Authorization: `Bearer ${token}` },
    data: {
      stage: "receive",
      result: "received",
      direction: "inbound",
      message_type: "asn",
      schema_version: "1",
      external_ref: "H8-MSG-E2E-PAYLOAD",
      idempotency_key: "h8-msg-e2e-payload-idem",
      correlation_id: "h8-msg-e2e-payload-corr",
      channel: "interface_table",
      connector_id: connectorId,
      connector_code: "H8-IF-E2E",
      config_version: 1,
      payload: { external_ref: "H8-MSG-E2E-PAYLOAD", product_name: "真实测试药品" },
    },
  });
  expect(lifecycle.status(), await lifecycle.text()).toBe(200);

  await page.getByRole("tab", { name: "消息记录" }).click();
  await page.getByRole("button", { name: "重置", exact: true }).click();
  await page.getByRole("button", { name: "展开", exact: true }).click();
  await page.getByLabel("外部业务标识", { exact: true }).fill("H8-MSG-E2E-PAYLOAD");
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const payloadRow = page.locator("tbody tr").filter({ hasText: "H8-MSG-E2E-PAYLOAD" });
  await expect(payloadRow).toBeVisible();
  await payloadRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText(/加密保留至/)).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "查看完整报文" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(1);
  await expect(page.getByRole("dialog").getByText(/真实测试药品/)).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "payload-decrypted-real.png"), fullPage: false });
});

test("H8 ERP 消息真实链路：只读权限与跨货主拒绝", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "wh-manager");
  const token = await accessToken(page);
  await openMessagesPage(page);
  await expect(page.getByRole("button", { name: "重放", exact: true })).toHaveCount(0);
  await page.getByRole("tab", { name: "Worker 状态" }).click();
  await expect(page.getByRole("button", { name: /暂停认领|恢复认领|报文保留/ })).toHaveCount(0);

  const crossOwner = await page.request.get(
    `${apiURL}/api/v1/integration/erp-messages/00000000-0000-0000-0000-000000008999`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  expect(crossOwner.status(), await crossOwner.text()).toBe(404);
  await page.screenshot({ path: path.join(screenshotDir, "readonly-real.png"), fullPage: false });
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

async function login(page: import("@playwright/test").Page, username: string) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const response = page.waitForResponse(
    (item) => item.url().endsWith("/api/v1/auth/login") && item.request().method() === "POST",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  expect((await response).status()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openMessagesPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await navigation.getByRole("button", { name: /ERP 消息/ }).click();
  await expect(page.getByRole("heading", { name: "H8 ERP 消息" })).toBeVisible();
}

async function accessToken(page: import("@playwright/test").Page): Promise<string> {
  const token = await page.evaluate(() => {
    const session = JSON.parse(localStorage.getItem("wms.web-admin.auth-session") ?? "null") as {
      accessToken?: string;
    } | null;
    return session?.accessToken ?? "";
  });
  expect(token.length).toBeGreaterThan(10);
  return token;
}
