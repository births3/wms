import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const apiURL = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19289";
const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/m10-real/screenshots");
const ownerId = "00000000-0000-0000-0000-000000000001";
const driverUserId = "00000000-0000-0000-0000-000000000101";
const outboundOrderId = "00000000-0000-0000-0000-000000001701";
const storeId = "00000000-0000-0000-0000-000000001201";
const routePlanPath = "/api/v1/tms/route-plans";

type MenuNode = { title: string; children: MenuNode[] };
type RoutePlan = {
  id: string;
  owner_id: string;
  dispatch_result_id: string;
  delivery_date: string;
  vehicle_no: string;
  plate_no: string;
  driver_user_id: string;
  status: string;
  version: number;
  outbound_order_ids: string[];
  stops: Array<{
    id: string;
    store_id: string;
    sequence: number;
    estimated_arrival_at: string;
    outbound_order_ids: string[];
  }>;
};

test("US-M10-001 真实后端接收路径规划结果并保留浏览器证据", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openRoutePlanPage(page);

  await expect(page.getByRole("heading", { name: "接收 TMS 路径规划结果", exact: true })).toBeVisible();
  for (const label of [
    "配送日期",
    "TMS 调度结果 ID",
    "司机 user_id",
    "规划版本",
    "车辆编号",
    "车牌号",
    "出库订单 ID",
    "路线站点 JSON",
  ]) {
    await expect(page.getByLabel(label, { exact: true })).toBeVisible();
  }
  await expect(page.getByRole("button", { name: "接收路线结果", exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "page-loaded.png"), fullPage: false });

  const dispatchResultId = `M10-E2E-ROUTE-${Date.now()}`;
  await fillRoutePlan(page, dispatchResultId, JSON.stringify(invalidStops()));
  const invalidResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(routePlanPath) && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "接收路线结果", exact: true }).click();
  const invalidResponse = await invalidResponsePromise;
  expect(invalidResponse.status()).toBe(422);
  await expect(page.getByRole("alert")).toContainText("HTTP 422");
  await page.screenshot({ path: path.join(artifactsDir, "invalid-stop-sequence.png"), fullPage: false });

  await page.getByLabel("路线站点 JSON", { exact: true }).fill(JSON.stringify(validStops()));
  const successResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(routePlanPath) && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "接收路线结果", exact: true }).click();
  const successResponse = await successResponsePromise;
  const successBody = await successResponse.text();
  expect(successResponse.status(), successBody).toBe(200);
  const routePlan = JSON.parse(successBody) as RoutePlan;
  expect(routePlan).toMatchObject({
    owner_id: ownerId,
    dispatch_result_id: dispatchResultId,
    delivery_date: "2026-07-14",
    vehicle_no: "VH-M10-E2E-001",
    plate_no: "沪A00001",
    driver_user_id: driverUserId,
    status: "received",
    version: 1,
    outbound_order_ids: [outboundOrderId],
  });
  expect(routePlan.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  expect(routePlan.stops).toHaveLength(1);
  expect(routePlan.stops[0]).toMatchObject({
    store_id: storeId,
    sequence: 1,
    outbound_order_ids: [outboundOrderId],
  });
  await expect(page.getByText("路线结果已接收", { exact: true })).toBeVisible();
  await expect(page.getByRole("status")).toContainText("已保存返回的路线信息");
  await expect(page.getByText(routePlan.id, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "route-plan-received.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.evaluate(() => localStorage.clear());
  await page.reload();
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  const loginResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/auth/login") && response.request().method() === "POST",
  );
  const publishedMenuResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/admin/menus/published") && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "登录", exact: true }).click();
  const loginResponse = await loginResponsePromise;
  expect(loginResponse.status(), await loginResponse.text()).toBe(200);
  await expect(page.getByRole("heading", { name: "运营总览", exact: true })).toBeVisible();

  const publishedMenuResponse = await publishedMenuResponsePromise;
  const publishedMenuBody = (await publishedMenuResponse.json()) as { data?: MenuNode[]; version_no?: number | null };
  expect(publishedMenuResponse.status(), JSON.stringify(publishedMenuBody)).toBe(200);
  expect(publishedMenuBody.version_no).toEqual(expect.any(Number));
  expect(hasMenuPath(publishedMenuBody.data ?? [], ["增值业务", "增值作业", "M10 路径规划接收"])).toBeTruthy();
}

async function openRoutePlanPage(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const section = navigation.getByRole("button", { name: "增值业务", exact: true });
  if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
  const group = navigation.getByRole("button", { name: "增值作业", exact: true });
  if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  const target = navigation.getByRole("button", { name: /M10 路径规划接收/ });
  await expect(target).toBeVisible();
  await target.click();
}

async function fillRoutePlan(page: import("@playwright/test").Page, dispatchResultId: string, stopsJson: string) {
  await page.getByLabel("配送日期", { exact: true }).fill("2026-07-14");
  await page.getByLabel("TMS 调度结果 ID", { exact: true }).fill(dispatchResultId);
  await page.getByLabel("司机 user_id", { exact: true }).fill(driverUserId);
  await page.getByLabel("规划版本", { exact: true }).fill("1");
  await page.getByLabel("车辆编号", { exact: true }).fill("VH-M10-E2E-001");
  await page.getByLabel("车牌号", { exact: true }).fill("沪A00001");
  await page.getByLabel("出库订单 ID", { exact: true }).fill(outboundOrderId);
  await page.getByLabel("路线站点 JSON", { exact: true }).fill(stopsJson);
}

function validStops() {
  return [{
    sequence: 1,
    store_id: storeId,
    estimated_arrival_at: "2026-07-14T09:00:00Z",
    outbound_order_ids: [outboundOrderId],
  }];
}

function invalidStops() {
  return [{ ...validStops()[0], sequence: 2 }];
}

function hasMenuPath(nodes: MenuNode[], titles: string[]) {
  let current = nodes;
  for (const title of titles) {
    const node = current.find((candidate) => candidate.title === title);
    if (!node) return false;
    current = node.children;
  }
  return true;
}
