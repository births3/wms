import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

test("M-TE 任务类型配置使用真实 API 展示预置类型并保存自定义类型", async ({ page }) => {
  await login(page);
  await openTaskTypePage(page);
  await expect(page.getByRole("heading", { name: "M-TE 任务类型配置" })).toBeVisible();
  await expect(page.locator("tbody tr").first()).toBeVisible();
  const code = `e2e_${Date.now()}`;
  await page.getByRole("button", { name: "新增类型", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增任务类型" });
  await dialog.getByLabel("类型编码").fill(code);
  await dialog.getByLabel("类型名称").fill("E2E 自定义任务");
  await dialog.getByLabel("释放策略").selectOption("scheduled");
  await dialog.getByLabel("释放间隔（分）").fill("10");
  await dialog.getByLabel("每批任务数").fill("5");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("E2E 自定义任务")).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: code });
  await expect(row).toContainText("定时释放");
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog", { name: "停用任务类型" }).getByRole("button", { name: "确认", exact: true }).click();
  await expect(row).toContainText("停用");
  await page.getByRole("button", { name: "配置优先级规则", exact: true }).click();
  const priorityDialog = page.getByRole("dialog", { name: "配置任务优先级规则" });
  await priorityDialog.getByLabel("订单加急加分").fill("30");
  await priorityDialog.getByLabel("等待多少分钟加 1 分").fill("5");
  await priorityDialog.getByLabel("冷链任务加分").fill("20");
  await priorityDialog.getByLabel("手动加急加分").fill("40");
  await priorityDialog.getByRole("button", { name: "保存规则", exact: true }).click();
  await expect(page.getByText(/订单加急 \+30 · 每等待 5 分钟 \+1 · 冷链 \+20 · 手动加急 \+40/)).toBeVisible();
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/m-te-task-types");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "task-type-config.png"), fullPage: false });
});

test("M-TE 任务组和调度使用真实 API 完成创建、自动分派与下发", async ({ page }) => {
  const browserNow = Date.now();
  await page.clock.install({ time: new Date(browserNow) });
  await login(page);
  await configurePriorityRule(page);
  await configurePutawayRelease(page, "scheduled");
  await assertTaskEngineReads(page);
  await openTaskGroupPage(page);
  await expect(page.getByRole("heading", { name: "M-TE 任务组与人员资格" })).toBeVisible();

  const suffix = Date.now();
  const groupCode = `e2e_group_${suffix}`;
  await page.getByRole("button", { name: "新增任务组", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新增任务组" });
  await dialog.getByLabel("任务组编码").fill(groupCode);
  await dialog.getByLabel("任务组名称").fill("E2E 调度组");
  await dialog.getByLabel("适用仓库").click();
  await page.getByRole("option", { name: /WH-M1-E2E-001/ }).click();
  await dialog.getByRole("checkbox", { name: "上架（putaway）" }).check();
  await dialog.getByRole("checkbox", { name: "系统管理员（admin）" }).check();
  const validUntil = new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString().slice(0, 16);
  await dialog.getByLabel("系统管理员 资格有效期").fill(validUntil);
  await dialog.getByLabel("系统管理员 同时在手上限").fill("2");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText(`任务组 ${groupCode} 已保存`)).toBeVisible();

  const task = await page.evaluate(async ({ taskGroupCode, sourceKey }) => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const response = await fetch("/api/v1/task-engine/tasks", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `e2e-task-${sourceKey}`,
      },
      body: JSON.stringify({
        task_type_code: "putaway",
        source_module: "M2",
        source_doc_type: "e2e_receiving",
        source_doc_id: null,
        source_doc_no: `E2E-MTE-${sourceKey}`,
        source_line_no: 1,
        source_task_key: `e2e-mte-${sourceKey}`,
        warehouse_id: "00000000-0000-0000-0000-000000001301",
        task_group_code: taskGroupCode,
        product_id: null,
        product_code: "P-MTE-E2E-001",
        batch_id: null,
        batch_no: "B-MTE-E2E-001",
        planned_qty: 5,
        source_location_id: null,
        source_location_code: "RECV",
        target_location_id: null,
        target_location_code: "A01-01-01",
        priority: 80,
        urgent_order: true,
      }),
    });
    if (!response.ok) throw new Error(`create task failed: ${response.status} ${await response.text()}`);
    return response.json() as Promise<{ task_no: string; source_doc_no: string }>;
  }, { taskGroupCode: groupCode, sourceKey: String(suffix) });
  await configurePutawayRelease(page, "immediate");

  await openTaskDispatchPage(page);
  await expect(page.getByRole("heading", { name: "M-TE 任务调度" })).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: task.source_doc_no });
  await expect(row).toContainText("待释放");
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "释放", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(row).toContainText("待分配");
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await expect(row.getByText("110", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "手动加急", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(page.getByText(/已手动加急/)).toBeVisible();
  await expect(row.getByText("150", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "自动分派", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(row).toContainText("已分配");
  await page.getByRole("button", { name: "下发", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(row).toContainText("已下发");
  await page.clock.setSystemTime(new Date(browserNow + 24 * 60 * 60 * 1000));
  await page.getByLabel("自动刷新").selectOption("5000");
  await expect(row).toContainText("未接单超时");

  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/m-te-task-execution");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "task-dispatched.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) { await page.goto("/"); await page.getByLabel("货主编码").fill("PY_OWNER"); await page.getByLabel("登录账号").fill("admin"); await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!"); await page.getByRole("button", { name: "登录" }).click(); await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible(); }
async function assertTaskEngineReads(page: import("@playwright/test").Page) {
  const failures = await page.evaluate(async () => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const paths = ["/api/v1/task-engine/task-groups", "/api/v1/task-engine/workers"];
    const responses = await Promise.all(paths.map(async (path) => {
      const response = await fetch(path, { headers: { Authorization: `Bearer ${token}` } });
      return response.ok ? null : `${path}: ${response.status} ${await response.text()}`;
    }));
    return responses.filter(Boolean);
  });
  expect(failures, failures.join("\n")).toEqual([]);
}
async function configurePriorityRule(page: import("@playwright/test").Page) {
  const failure = await page.evaluate(async () => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const response = await fetch("/api/v1/task-engine/priority-rule", {
      method: "PUT",
      headers: { Authorization: `Bearer ${token}`, "Content-Type": "application/json", "Idempotency-Key": `e2e-priority-${Date.now()}` },
      body: JSON.stringify({ urgent_order_bonus: 30, waiting_minutes_per_point: 5, cold_chain_bonus: 20, manual_expedite_bonus: 40 }),
    });
    return response.ok ? null : `${response.status} ${await response.text()}`;
  });
  expect(failure).toBeNull();
}
async function configurePutawayRelease(page: import("@playwright/test").Page, strategy: "immediate" | "scheduled") {
  const failure = await page.evaluate(async (releaseStrategy) => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    const token = raw ? (JSON.parse(raw) as { accessToken: string }).accessToken : "";
    const listResponse = await fetch("/api/v1/task-engine/task-types", { headers: { Authorization: `Bearer ${token}` } });
    if (!listResponse.ok) return `list task types failed: ${listResponse.status} ${await listResponse.text()}`;
    const list = await listResponse.json() as { data: Array<Record<string, unknown>> };
    const taskType = list.data.find((item) => item.task_type_code === "putaway");
    if (!taskType) return "putaway task type not found";
    const response = await fetch("/api/v1/task-engine/task-types/putaway", {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "Idempotency-Key": `e2e-putaway-release-${Date.now()}`,
      },
      body: JSON.stringify({
        task_type_name: taskType.task_type_name,
        default_priority: taskType.default_priority,
        estimated_minutes: taskType.estimated_minutes,
        mergeable: taskType.mergeable,
        insertable: taskType.insertable,
        enabled: taskType.enabled,
        release_strategy: releaseStrategy,
        release_interval_minutes: releaseStrategy === "scheduled" ? 10 : null,
        release_batch_size: releaseStrategy === "scheduled" ? 5 : null,
      }),
    });
    return response.ok ? null : `configure release failed: ${response.status} ${await response.text()}`;
  }, strategy);
  expect(failure).toBeNull();
}
async function openTaskTypePage(page: import("@playwright/test").Page) {
  await openInventoryPage(page, /M-TE 任务类型配置/);
}
async function openTaskGroupPage(page: import("@playwright/test").Page) {
  await openInventoryPage(page, /M-TE 任务组资格/);
}
async function openTaskDispatchPage(page: import("@playwright/test").Page) {
  await openInventoryPage(page, /M-TE 任务调度/);
}
async function openInventoryPage(page: import("@playwright/test").Page, targetName: RegExp) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: targetName });
  const section = navigation.getByRole("button", { name: "库内业务", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: "库存管理", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  await expect(target).toBeVisible();
  await target.click();
}
