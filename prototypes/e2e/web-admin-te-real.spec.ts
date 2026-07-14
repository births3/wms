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
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.getByText("E2E 自定义任务")).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: code });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "停用", exact: true }).click();
  await page.getByRole("dialog", { name: "停用任务类型" }).getByRole("button", { name: "确认", exact: true }).click();
  await expect(row).toContainText("停用");
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  const screenshotDir = path.join(repoRoot, "artifacts/screenshot-portal/real-web/m-te-task-types");
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({ path: path.join(screenshotDir, "task-type-config.png"), fullPage: false });
});

test("M-TE 任务组和调度使用真实 API 完成创建、自动分派与下发", async ({ page }) => {
  await login(page);
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
      }),
    });
    if (!response.ok) throw new Error(`create task failed: ${response.status} ${await response.text()}`);
    return response.json() as Promise<{ task_no: string; source_doc_no: string }>;
  }, { taskGroupCode: groupCode, sourceKey: String(suffix) });

  await openTaskDispatchPage(page);
  await expect(page.getByRole("heading", { name: "M-TE 任务调度" })).toBeVisible();
  const row = page.locator("tbody tr").filter({ hasText: task.source_doc_no });
  await expect(row).toContainText("待分配");
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "自动分派", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(row).toContainText("已分配");
  await page.getByRole("button", { name: "下发", exact: true }).click();
  await page.getByRole("dialog", { name: "确认任务操作" }).getByRole("button", { name: "确认执行" }).click();
  await expect(row).toContainText("已下发");

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
