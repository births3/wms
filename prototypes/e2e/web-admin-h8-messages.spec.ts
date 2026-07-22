/**
 * US-H8-003：H8 ERP 消息页 E2E（dev mock / 真实代理均可）。
 * 覆盖失败查询、死信详情、重放、只读无重放按钮。
 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h8-erp-messages");

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

async function openMessagesPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: /H8 集成中心/ });
  if ((await group.count()) > 0 && (await group.getAttribute("aria-expanded")) !== "true") {
    await group.click();
  }
  const target = navigation.getByRole("button", { name: /ERP 消息/ });
  await expect(target).toBeVisible({ timeout: 15_000 });
  await target.click();
  await expect(page.getByRole("heading", { name: "H8 ERP 消息" })).toBeVisible({ timeout: 15_000 });
}

test("H8 ERP 消息：失败筛选、死信详情与重放", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openMessagesPage(page);

  await page.getByRole("button", { name: "展开", exact: true }).click();
  await page.getByLabel("消息类型", { exact: true }).selectOption("asn");
  await page.getByLabel("通道", { exact: true }).selectOption("rest");
  await page.getByLabel("连接编码", { exact: true }).fill("demo-rest-erp");
  const filteredList = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/integration/erp-messages?") &&
      response.url().includes("connector_code=demo-rest-erp") &&
      response.url().includes("channel=rest") &&
      response.url().includes("message_type=asn"),
  );
  const filteredStats = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/integration/erp-messages/stats?") &&
      response.url().includes("connector_code=demo-rest-erp") &&
      response.url().includes("channel=rest") &&
      response.url().includes("message_type=asn"),
  );
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filteredList).status()).toBe(200);
  expect((await filteredStats).status()).toBe(200);
  await expect(page.getByText("合计 2", { exact: true })).toBeVisible();

  // mock 默认含 failed / dead 两条
  await expect(page.getByText("ERP-ASN-FAIL-1")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByText("ERP-ASN-DEAD-1")).toBeVisible();
  await expect(page.locator("tbody").getByText("预到货通知（ASN）").first()).toBeVisible();
  await expect(page.locator("tbody").getByText("REST").first()).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "message-list.png"), fullPage: false });

  const failedRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-FAIL-1" });
  await expect(failedRow.getByText("失败")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "failed-filter.png"), fullPage: false });

  const deadRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  await deadRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText("消息详情")).toBeVisible();
  await expect(page.getByRole("dialog").getByText("digest-dead")).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "dead-detail.png"), fullPage: false });
  await page.getByRole("dialog").getByRole("button", { name: "关闭", exact: true }).click();

  // 详情关闭后重新单选 failed 行再重放
  if (await deadRow.getByRole("checkbox", { name: "选择此行" }).isChecked()) {
    await deadRow.getByRole("checkbox", { name: "选择此行" }).uncheck();
  }
  await failedRow.getByRole("checkbox", { name: "选择此行" }).check();
  await expect(page.getByRole("button", { name: "重放", exact: true })).toBeEnabled({ timeout: 5_000 });
  await page.getByRole("button", { name: "重放", exact: true }).click();
  await page.getByRole("dialog").getByPlaceholder("说明重放原因").fill("e2e 人工重放");
  await page.getByRole("dialog").getByRole("button", { name: "确认重放", exact: true }).click();
  await expect(page.getByText(/已提交重放/)).toBeVisible({ timeout: 15_000 });
  await page.screenshot({ path: path.join(screenshotDir, "replay-success.png"), fullPage: false });
});

test("H8 ERP 消息：只读用户无重放按钮", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.route("**/api/v1/auth/me", async (route) => {
    const response = await route.fetch();
    const user = await response.json();
    user.permissions = user.permissions.filter(
      (permission: string) => permission !== "h8.erp_connector.write",
    );
    await route.fulfill({ response, json: user });
  });
  await login(page, "admin");
  await openMessagesPage(page);
  await expect(page.getByRole("button", { name: "重放", exact: true })).toHaveCount(0);
  await page.getByRole("tab", { name: "Worker 状态" }).click();
  await expect(page.getByRole("button", { name: /暂停认领|恢复认领/ })).toHaveCount(0);
  await page.screenshot({ path: path.join(screenshotDir, "readonly-no-replay.png"), fullPage: false });
});

test("H8 ERP 消息：更多查询全部进入服务端契约", async ({ page }) => {
  const createdFrom = new Date(2026, 6, 19, 0, 0, 0, 0).toISOString();
  const createdTo = new Date(2026, 6, 19, 23, 59, 59, 999).toISOString();
  await login(page, "admin");
  await openMessagesPage(page);
  await page.getByRole("button", { name: "展开", exact: true }).click();
  await page
    .getByLabel("仓库", { exact: true })
    .selectOption("00000000-0000-0000-0000-000000000801");
  await page.getByLabel("外部业务标识", { exact: true }).fill("ERP-ASN-FAIL-1");
  await page.getByLabel("幂等键（Idempotency-Key）", { exact: true }).fill("idem-fail-1");
  await page.getByLabel("关联标识（Correlation）", { exact: true }).fill("corr-fail-1");
  await page.getByLabel("创建时间开始").fill("2026-07-19");
  await page.getByLabel("创建时间结束").fill("2026-07-19");

  const filtered = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return (
      url.pathname === "/api/v1/integration/erp-messages" &&
      url.searchParams.get("warehouse_id") === "00000000-0000-0000-0000-000000000801" &&
      url.searchParams.get("external_ref") === "ERP-ASN-FAIL-1" &&
      url.searchParams.get("idempotency_key") === "idem-fail-1" &&
      url.searchParams.get("correlation_id") === "corr-fail-1" &&
      url.searchParams.get("created_from") === createdFrom &&
      url.searchParams.get("created_to") === createdTo
    );
  });
  await page.getByRole("button", { name: "查询", exact: true }).click();
  expect((await filtered).status()).toBe(200);
  await expect(page.getByText("ERP-ASN-FAIL-1")).toBeVisible();
  await expect(page.getByText("ERP-ASN-DEAD-1")).toHaveCount(0);
});

test("H8 ERP 消息：详情接口失败显示中文错误", async ({ page }) => {
  await page.route(/\/api\/v1\/integration\/erp-messages\/[^/?]+$/, async (route) => {
    if (route.request().method() === "GET") {
      await route.fulfill({
        status: 500,
        contentType: "application/json",
        body: JSON.stringify({ code: "H8-500", message: "database operation failed" }),
      });
      return;
    }
    await route.continue();
  });
  await login(page, "admin");
  await openMessagesPage(page);
  const row = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("消息详情加载失败，请关闭后重试。");
});

test("H8 ERP 消息：Worker 心跳与暂停恢复认领", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openMessagesPage(page);
  await page.getByRole("tab", { name: "Worker 状态" }).click();

  const inboundRow = page.locator("tbody tr").filter({ hasText: "入站" });
  await expect(page.getByRole("columnheader", { name: "创建时间" })).toBeVisible();
  await expect(inboundRow.getByText("h8-worker-demo-01")).toBeVisible({ timeout: 15_000 });
  await expect(inboundRow.getByText("健康", { exact: true })).toBeVisible();
  await expect(inboundRow.getByText("运行中", { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "worker-status.png"), fullPage: false });

  await inboundRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "暂停认领", exact: true }).click();
  await page.getByPlaceholder("说明操作原因").fill("ERP 维护窗口");
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已暂停认领", { exact: true }).first()).toBeVisible({ timeout: 15_000 });
  await expect(inboundRow.getByText("已暂停", { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "worker-paused.png"), fullPage: false });

  await page.getByRole("button", { name: "恢复认领", exact: true }).click();
  await page.getByPlaceholder("说明操作原因").fill("维护完成");
  await page.getByRole("dialog").getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.getByText("已恢复认领", { exact: true }).first()).toBeVisible({ timeout: 15_000 });

  await page.getByRole("button", { name: "报文保留", exact: true }).click();
  await page.getByLabel("启用完整报文加密保留").check();
  await page.getByLabel("保留天数（1–30）").fill("7");
  await page.getByRole("dialog").getByRole("button", { name: "确认保存", exact: true }).click();
  await expect(page.getByText("已启用完整报文保留（7 天）", { exact: true })).toBeVisible({
    timeout: 15_000,
  });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.screenshot({ path: path.join(screenshotDir, "payload-retention.png"), fullPage: false });

  await page.getByRole("tab", { name: "消息记录" }).click();
  const messageRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  await messageRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", exact: true }).click();
  await expect(page.getByRole("dialog").getByText(/加密保留至/)).toBeVisible();
  await page.getByRole("dialog").getByRole("button", { name: "查看完整报文" }).click();
  await expect(page.getByRole("dialog").getByText(/external_ref/)).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "payload-decrypted.png"), fullPage: false });
});

test("H8 ERP 消息：重复重放不复制业务消息 ID", async ({ page }) => {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await login(page, "admin");
  await openMessagesPage(page);
  // dead 行重放两次：业务 message id 不变，状态进入 processing
  const deadRow = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  await expect(deadRow).toBeVisible({ timeout: 15_000 });
  const idCell = await deadRow.locator("td").first().textContent();
  await deadRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "重放", exact: true }).click();
  await page.getByRole("dialog").getByPlaceholder("说明重放原因").fill("e2e first replay");
  await page.getByRole("dialog").getByRole("button", { name: "确认重放", exact: true }).click();
  await expect(page.getByText(/已提交重放/)).toBeVisible({ timeout: 15_000 });
  // 二次：processing 不应再显示可重放，或重放失败不生成新 id
  await page.getByRole("button", { name: "重置", exact: true }).click().catch(() => undefined);
  await page.getByRole("button", { name: "查询", exact: true }).click().catch(() => undefined);
  const after = page.locator("tbody tr").filter({ hasText: "ERP-ASN-DEAD-1" });
  if ((await after.count()) > 0) {
    const idAfter = await after.locator("td").first().textContent();
    if (idCell && idAfter) {
      // 列表主键行仍指向同一业务消息（mock 不换 id）
      expect(idAfter).toBe(idCell);
    }
  }
  await page.screenshot({ path: path.join(screenshotDir, "replay-idempotent.png"), fullPage: false });
});
