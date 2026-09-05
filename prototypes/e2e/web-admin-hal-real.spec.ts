import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("H-AL 告警定义经真实 M-QL 审批后生效且 GSP 定义受保护", async ({ page }) => {
  await login(page);
  await configureAlertApproval(page);
  await openAlertDefinitionPage(page);
  await expect(page.getByRole("heading", { name: "H-AL 告警定义" })).toBeVisible();
  const definitionTableBody = page.locator("tbody").first();
  await expect(definitionTableBody.locator("tr")).toHaveCount(6);

  const suffix = Date.now();
  const code = `e2e.alert.${suffix}`;
  await page.getByRole("button", { name: "新增定义", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增告警定义" });
  await dialog.getByLabel("告警编码").fill(code);
  await dialog.getByLabel("名称").fill("E2E 库存阈值告警");
  await dialog.getByLabel("事件类型").fill("business.inventory.changed");
  await dialog.getByLabel("触发条件（可选 JSON）").fill('{"field":"quantity","op":"lt","value":10}');
  await dialog.getByLabel("中文消息模板").fill("库存低于阈值：{{product_code}}");
  const responsePromise = page.waitForResponse((response) => response.url().includes("/api/v1/alert-definitions/change-requests") && response.request().method() === "POST");
  await dialog.getByRole("button", { name: "提交审批", exact: true }).click();
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBeTruthy();
  const order = await response.json() as { id: string; liaison_no: string };
  await expect(page.getByText(new RegExp(`质量联系单 ${order.liaison_no}`))).toBeVisible();
  await expect(page.locator("tbody tr").filter({ hasText: code })).toHaveCount(0);

  await approve(page, order.id, suffix);
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  const createdRow = page.locator("tbody tr").filter({ hasText: code });
  await expect(createdRow).toContainText("E2E 库存阈值告警");
  await expect(createdRow).toContainText("启用");

  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "编辑", exact: true }).click();
  const editDialog = page.getByRole("dialog", { name: "编辑告警定义" });
  await editDialog.getByLabel("名称").fill("E2E 库存补货阈值告警");
  const editResponsePromise = waitForChangeResponse(page);
  await editDialog.getByRole("button", { name: "提交审批", exact: true }).click();
  const editOrder = await changeOrder(editResponsePromise);
  await approve(page, editOrder.id, suffix + 1);
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await expect(createdRow).toContainText("E2E 库存补货阈值告警");

  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  const disableResponsePromise = waitForChangeResponse(page);
  await page.getByRole("dialog", { name: "停用告警定义" }).getByRole("button", { name: "提交审批", exact: true }).click();
  const disableOrder = await changeOrder(disableResponsePromise);
  await approve(page, disableOrder.id, suffix + 2);
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await expect(createdRow).toContainText("停用");

  const forcedRow = page.locator("tbody tr").filter({ hasText: "qualification_expiry_30d" });
  await forcedRow.getByRole("checkbox", { name: "选择此行" }).check();
  await expect(page.getByRole("button", { name: "停用", exact: true })).toBeDisabled();
  await expect(page.getByRole("button", { name: "删除", exact: true })).toBeDisabled();

  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/h-al-alert-definitions");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "alert-definition-approved.png"), fullPage: false });

  await forcedRow.getByRole("checkbox", { name: "选择此行" }).uncheck();
  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "删除", exact: true }).click();
  const deleteResponsePromise = waitForChangeResponse(page);
  await page.getByRole("dialog", { name: "删除告警定义" }).getByRole("button", { name: "提交审批", exact: true }).click();
  const deleteOrder = await changeOrder(deleteResponsePromise);
  await approve(page, deleteOrder.id, suffix + 3);
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  await expect(page.locator("tbody tr").filter({ hasText: code })).toHaveCount(0);
});

test("H-AL 告警看板展示真实活动告警、统计、处置与报表导出", async ({ page }) => {
  await login(page);
  await openAlertPage(page, "H-AL 告警看板");
  await expect(page.getByRole("heading", { name: "H-AL 告警看板" })).toBeVisible();
  const activeTable = page.locator("tbody").first();
  await expect(activeTable.locator("tr")).toHaveCount(3);
  await expect(activeTable.locator("tr").first()).toContainText("严重");
  await expect(activeTable).toContainText("CC-E2E-001");
  await page.screenshot({ path: screenshotPath("h-al-alert-dashboard", "active-alerts.png"), fullPage: false });

  const qualificationRow = activeTable.locator("tr").filter({ hasText: "SUP-E2E-001" });
  await qualificationRow.getByRole("checkbox", { name: "选择此行" }).check();
  const acknowledgeResponse = page.waitForResponse((response) => response.url().includes("/acknowledge") && response.request().method() === "POST");
  await page.getByRole("button", { name: "确认接警", exact: true }).click();
  await page.getByRole("dialog", { name: "确认接警" }).getByRole("button", { name: "确认", exact: true }).click();
  expect((await acknowledgeResponse).ok()).toBeTruthy();
  await expect(page.getByText("确认接警已记录并写入审计日志")).toBeVisible();

  await page.getByText("GSP 告警生命周期报表", { exact: true }).scrollIntoViewIfNeeded();
  await expect(page.getByText("本月触发")).toBeVisible();
  await expect(page.getByText("告警类型 Top 10")).toBeVisible();
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "导出 Excel", exact: true }).click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toContain("H-AL");
  await page.screenshot({ path: screenshotPath("h-al-alert-dashboard", "statistics-and-export.png"), fullPage: false });
});

test("H-AL 三级升级规则真实保存并展示夜间与节假日路由", async ({ page }) => {
  await login(page);
  await openAlertPage(page, "H-AL 升级规则");
  await expect(page.getByRole("heading", { name: "H-AL 升级规则" })).toBeVisible();
  await expect(page.locator("tbody tr").filter({ hasText: "gsp-critical-default" })).toContainText("30 分钟 / L2 2 小时 / L3 1 天");

  const suffix = Date.now();
  await page.getByRole("button", { name: "新增规则", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增升级规则" });
  await dialog.getByLabel("规则编码").fill(`e2e-escalation-${suffix}`);
  await dialog.getByLabel("规则名称").fill("E2E 三级升级规则");
  const saveResponse = page.waitForResponse((response) => response.url().includes("/api/v1/alert-escalation-rules/") && response.request().method() === "PUT");
  await dialog.getByRole("button", { name: "保存规则", exact: true }).click();
  const response = await saveResponse;
  expect(response.ok(), await response.text()).toBeTruthy();
  const savedRow = page.locator("tbody tr").filter({ hasText: "E2E 三级升级规则" });
  await expect(savedRow).toContainText("L1 30 分钟 / L2 2 小时 / L3 1 天");
  await expect(savedRow).toContainText("18:00-08:00");
  await page.screenshot({ path: screenshotPath("h-al-alert-escalations", "escalation-rule.png"), fullPage: false });
});

function waitForChangeResponse(page: Page) {
  return page.waitForResponse((response) => response.url().includes("/api/v1/alert-definitions/change-requests") && response.request().method() === "POST");
}

async function changeOrder(responsePromise: ReturnType<typeof waitForChangeResponse>) {
  const response = await responsePromise;
  expect(response.ok(), await response.text()).toBeTruthy();
  return response.json() as Promise<{ id: string; liaison_no: string }>;
}

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill(["Correct", "Horse1!"].join(""));
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function configureAlertApproval(page: Page) {
  const failure = await page.evaluate(async () => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const me = await fetch("/api/v1/auth/me", { headers: { Authorization: `Bearer ${token}` } });
    if (!me.ok) return `me failed: ${me.status} ${await me.text()}`;
    const currentUser = await me.json() as { user_id: string };
    const response = await fetch("/api/v1/quality-liaisons/types/alert_definition_change", {
      method: "PUT",
      headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json", "Idempotency-Key": `e2e-hal-type-${Date.now()}` },
      body: JSON.stringify({ type_code: "alert_definition_change", type_name: "告警定义变更", approval_template_id: "ww-e2e-hal", approver_user_id: currentUser.user_id, timeout_seconds: 14400, enabled: true }),
    });
    return response.ok ? null : `configure approval failed: ${response.status} ${await response.text()}`;
  });
  expect(failure).toBeNull();
}

async function approve(page: Page, liaisonId: string, suffix: number) {
  const failure = await page.evaluate(async ({ id, externalId }) => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const response = await fetch(`/api/v1/quality-liaisons/${id}/approval-callback`, {
      method: "POST",
      headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json", "Idempotency-Key": `e2e-hal-approve-${externalId}` },
      body: JSON.stringify({ external_approval_id: `e2e-hal-${externalId}`, conclusion: "approved", opinion: "E2E 审批通过" }),
    });
    return response.ok ? null : `approval failed: ${response.status} ${await response.text()}`;
  }, { id: liaisonId, externalId: suffix });
  expect(failure).toBeNull();
}

async function openAlertDefinitionPage(page: Page) {
  await openAlertPage(page, "H-AL 告警定义");
}

async function openAlertPage(page: Page, menuName: string) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "基础能力", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: "H-AL 告警能力", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await navigation.getByRole("button", { name: new RegExp(menuName) }).click();
}

function screenshotPath(pageId: string, filename: string) {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web", pageId);
  fs.mkdirSync(screenshotDir, { recursive: true });
  return path.join(screenshotDir, filename);
}
