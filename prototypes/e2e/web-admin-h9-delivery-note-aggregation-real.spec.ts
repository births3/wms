import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const evidenceDir = path.resolve("../artifacts/screenshot-portal/real-web/h9-delivery-note-aggregation");
const orderNo = "OUT-H9-E2E-006";

test.use({ viewport: { width: 1600, height: 900 } });

test("US-H9-006 真实线路、计划与随货同行单归集", async ({ browser, page }) => {
  fs.mkdirSync(evidenceDir, { recursive: true });
  await login(page);
  await openDeliveryNoteAggregation(page);

  const candidateRow = page.getByRole("row").filter({ hasText: orderNo });
  await expect(candidateRow).toContainText("ERP-H9-E2E-006");
  await expect(candidateRow).toContainText("E2E 客户门店");
  await expect(candidateRow).toContainText("真实数据路 006 号");
  await expect(candidateRow).toContainText("LINE-H9-E2E-006");
  await page.screenshot({ path: path.join(evidenceDir, "pending-orders.png"), fullPage: false });

  await candidateRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "人工截单", exact: true }).click();
  const cutoffDialog = page.getByRole("dialog", { name: "授权人工截单" });
  await cutoffDialog.getByLabel("截单原因").fill("真实 E2E 装车前授权截单");
  const cutoffResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/delivery-note-groups/manual-cutoff")
      && response.request().method() === "POST",
  );
  await cutoffDialog.getByRole("button", { name: "确认截单" }).click();
  const cutoff = await cutoffResponse;
  expect(cutoff.ok(), await cutoff.text()).toBeTruthy();
  const group = await cutoff.json() as { delivery_note_no: string };
  await expect(page.getByRole("status")).toContainText(group.delivery_note_no);
  await page.getByRole("tab", { name: /截单结果/ }).click();
  const groupRow = page.getByRole("row").filter({ hasText: group.delivery_note_no });
  await expect(groupRow).toContainText(orderNo);
  await expect(groupRow).toContainText("人工截单");
  await expect(groupRow).toContainText("真实 E2E 装车前授权截单");
  await page.screenshot({ path: path.join(evidenceDir, "cutoff-result.png"), fullPage: false });

  await page.getByRole("tab", { name: /截单计划/ }).click();
  const seededPlan = page.getByRole("row").filter({ hasText: "E2E 客户截单计划" });
  await expect(seededPlan).toContainText("客户");
  await expect(seededPlan).toContainText("周一 17:00");
  await expect(seededPlan).toContainText("2026-08-01 12:00");
  await page.getByRole("button", { name: "新建计划", exact: true }).click();
  const planDialog = page.getByRole("dialog", { name: "新建截单计划" });
  const planName = `E2E 线路截单计划 ${Date.now()}`;
  await planDialog.getByLabel("计划名称").fill(planName);
  await planDialog.getByLabel("适用层级").selectOption("route");
  await planDialog.getByLabel("线路编码").fill("LINE-H9-E2E-006");
  const createPlanResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/cutoff-plans")
      && response.request().method() === "POST",
  );
  await planDialog.getByRole("button", { name: "保存草稿" }).click();
  expect((await createPlanResponse).ok()).toBeTruthy();
  const planRow = page.getByRole("row").filter({ hasText: planName });
  await expect(planRow).toContainText("草稿");
  await planRow.getByRole("checkbox", { name: "选择此行" }).check();
  page.once("dialog", (dialog) => dialog.accept());
  const publishPlanResponse = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/print-orchestration/cutoff-plans/")
      && response.url().endsWith("/publish")
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "发布计划", exact: true }).click();
  expect((await publishPlanResponse).ok()).toBeTruthy();
  await expect(planRow).toContainText("已发布");
  await page.screenshot({ path: path.join(evidenceDir, "cutoff-plans.png"), fullPage: false });

  await page.getByRole("tab", { name: /线路绑定/ }).click();
  const seededRoute = page.getByRole("row").filter({ hasText: "LINE-H9-E2E-006" });
  await expect(seededRoute).toContainText("E2E 客户门店");
  await expect(seededRoute).toContainText("上海市上海市浦东新区真实数据路 006 号");
  await page.getByRole("button", { name: "发布线路", exact: true }).click();
  const routeDialog = page.getByRole("dialog", { name: "发布送货地址线路" });
  await routeDialog.getByLabel("线路编码").fill("LINE-H9-E2E-NEXT");
  await routeDialog.getByLabel("生效时间").fill("2100-01-02T00:00");
  const publishRouteResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith("/api/v1/print-orchestration/route-bindings")
      && response.request().method() === "POST",
  );
  await routeDialog.getByRole("button", { name: "发布线路" }).click();
  expect((await publishRouteResponse).ok()).toBeTruthy();
  await expect(page.getByRole("row").filter({ hasText: "LINE-H9-E2E-NEXT" })).toBeVisible();
  await page.screenshot({ path: path.join(evidenceDir, "plans-and-routes.png"), fullPage: false });

  const refreshedContext = await browser.newContext({ viewport: { width: 1600, height: 900 } });
  const refreshedPage = await refreshedContext.newPage();
  await login(refreshedPage);
  await openDeliveryNoteAggregation(refreshedPage);
  await refreshedPage.getByRole("tab", { name: /线路绑定/ }).click();
  await expect(refreshedPage.getByRole("row").filter({ hasText: "LINE-H9-E2E-NEXT" })).toBeVisible();
  await refreshedContext.close();
});

async function login(page: Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openDeliveryNoteAggregation(page: Page) {
  await page.getByRole("button", { name: "基础能力", exact: true }).click();
  await page.getByRole("button", { name: "H9 打印能力", exact: true }).click();
  await page.getByRole("button", { name: /作业·随货同行单归集/ }).click();
  await expect(page.getByRole("heading", { name: "作业·随货同行单归集" })).toBeVisible();
}
