import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m4-outbound");

test("M4 PC 新建出库单使用真实 API 返回单据类型和自动单号", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openOutboundOrders(page);

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "新建出库单" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("ERP 单号").fill(`ERP-M4-E2E-${Date.now()}`);
  await dialog.getByLabel("单据类型").selectOption("sales_outbound");
  await dialog.getByLabel("商品编码").fill("P-M1-E2E-001");
  await dialog.getByLabel("批号").fill("B-M4-E2E-001");
  await dialog.getByLabel("计划数量").fill("8");

  const responsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/orders") && response.request().method() === "POST",
  );
  await dialog.getByRole("button", { name: "创建出库单" }).click();
  const response = await responsePromise;
  const responseBody = await response.text();
  expect(response.ok(), `M4 create returned ${response.status()}: ${responseBody}`).toBeTruthy();
  const created = JSON.parse(responseBody) as { id: string; wms_order_no: string; document_type: string; lines: Array<{ planned_qty: number }> };
  expect(created.document_type).toBe("sales_outbound");
  expect(created.wms_order_no).toMatch(/OUT|SO|销售|M4/i);
  expect(created.lines[0]?.planned_qty).toBe(8);

  await expect(page.getByRole("status")).toContainText(`${created.wms_order_no} 已创建`);
  await expect(page.getByText(created.wms_order_no, { exact: true })).toBeVisible();
  const refreshResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/orders") && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  const refreshResponse = await refreshResponsePromise;
  expect(refreshResponse.ok()).toBeTruthy();
  await expect(page.getByText(created.wms_order_no, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-order-created.png"), fullPage: false });

  const createdRow = page.getByRole("row").filter({ hasText: created.wms_order_no }).first();
  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  const detailResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/outbound/orders/${created.id}`) && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "详情", exact: true }).click();
  const detailResponse = await detailResponsePromise;
  expect(detailResponse.ok()).toBeTruthy();
  const detailDialog = page.getByRole("dialog", { name: "订单详情" });
  await expect(detailDialog).toBeVisible();
  await expect(detailDialog.getByText(created.wms_order_no, { exact: true }).first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-order-detail.png"), fullPage: false });

  await page.keyboard.press("Escape");
  const wavesListResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/waves") && response.request().method() === "GET",
  );
  await openOutboundWaves(page);
  const wavesListResponse = await wavesListResponsePromise;
  expect(wavesListResponse.ok()).toBeTruthy();
  await page.getByRole("button", { name: "新增", exact: true }).click();
  const waveDialog = page.getByRole("dialog", { name: "新建波次" });
  await expect(waveDialog).toBeVisible();
  const waveResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/waves") && response.request().method() === "POST",
  );
  await waveDialog.getByRole("button", { name: "创建波次" }).click();
  const waveResponse = await waveResponsePromise;
  const waveResponseBody = await waveResponse.text();
  expect(waveResponse.ok(), `M4 wave create returned ${waveResponse.status()}: ${waveResponseBody}`).toBeTruthy();
  const createdWave = JSON.parse(waveResponseBody) as { id: string; wave_no: string; status: string; order_ids: string[] };
  expect(createdWave.status).toBe("released");
  expect(createdWave.order_ids).toContain(created.id);
  await expect(page.getByRole("status")).toContainText(`${createdWave.wave_no} 已创建`);
  await expect(page.getByText(createdWave.wave_no, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-wave-created.png"), fullPage: false });

  const refreshWavesResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/waves") && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "刷新", exact: true }).click();
  const refreshWavesResponse = await refreshWavesResponsePromise;
  expect(refreshWavesResponse.ok()).toBeTruthy();
  await expect(page.getByText(createdWave.wave_no, { exact: true })).toBeVisible();

  const createdWaveRow = page.getByRole("row").filter({ hasText: createdWave.wave_no }).first();
  await createdWaveRow.getByRole("checkbox", { name: "选择此行" }).check();
  const waveDetailResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/outbound/waves/${createdWave.id}`) && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "详情", exact: true }).click();
  const waveDetailResponse = await waveDetailResponsePromise;
  expect(waveDetailResponse.ok()).toBeTruthy();
  const waveDetailDialog = page.getByRole("dialog", { name: "波次详情" });
  await expect(waveDetailDialog).toBeVisible();
  await expect(waveDetailDialog.getByText(createdWave.wave_no, { exact: true }).first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-wave-detail.png"), fullPage: false });

  await page.keyboard.press("Escape");
  const cancelResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/outbound/waves/${createdWave.id}/cancel`) && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "取消", exact: true }).click();
  const cancelDialog = page.getByRole("dialog", { name: "取消波次" });
  await expect(cancelDialog).toBeVisible();
  const cancelButton = cancelDialog.getByRole("button", { name: "确认取消", exact: true });
  await cancelButton.click();
  const cancelResponse = await cancelResponsePromise;
  const cancelResponseBody = await cancelResponse.text();
  expect(cancelResponse.ok(), `M4 wave cancel returned ${cancelResponse.status()}: ${cancelResponseBody}`).toBeTruthy();
  const cancelledWave = JSON.parse(cancelResponseBody) as { id: string; wave_no: string; status: string };
  expect(cancelledWave.id).toBe(createdWave.id);
  expect(cancelledWave.status).toBe("cancelled");
  await expect(page.getByRole("status")).toContainText(`${createdWave.wave_no} 已取消`);
  await expect(page.getByText(createdWave.wave_no, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-wave-cancelled.png"), fullPage: false });
});

test("M4 PC 复核使用真实详情和提交 API", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openOutboundReview(page);

  const reviewOrderNo = "OUT-M4-REVIEW-E2E-001";
  const reviewOrderId = "00000000-0000-0000-0000-000000001702";
  const row = page.getByRole("row").filter({ hasText: reviewOrderNo }).first();
  await expect(row).toBeVisible();
  await row.getByRole("checkbox", { name: "选择此行" }).check();

  const detailResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/outbound/orders/${reviewOrderId}/review`) && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "复核", exact: true }).click();
  const detailResponse = await detailResponsePromise;
  const detailBody = await detailResponse.text();
  expect(detailResponse.ok(), `M4 review detail returned ${detailResponse.status()}: ${detailBody}`).toBeTruthy();
  const reviewDialog = page.getByRole("dialog", { name: "复核" });
  await expect(reviewDialog).toBeVisible();
  await expect(reviewDialog).toContainText("P-M4-REVIEW-E2E-001");
  await expect(reviewDialog).toContainText("M-VR：双人扫码");
  await reviewDialog.getByLabel("第二复核员用户 ID").fill("00000000-0000-4000-8000-000000000104");
  await page.screenshot({ path: path.join(artifactsDir, "outbound-review-detail.png"), fullPage: false });

  const submitResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/outbound/orders/${reviewOrderId}/review`) && response.request().method() === "POST",
  );
  await reviewDialog.getByRole("button", { name: "提交复核", exact: true }).click();
  const submitResponse = await submitResponsePromise;
  const submitBody = await submitResponse.text();
  expect(submitResponse.ok(), `M4 review submit returned ${submitResponse.status()}: ${submitBody}`).toBeTruthy();
  const reviewed = JSON.parse(submitBody) as { id: string; status: string };
  expect(reviewed.id).toBe(reviewOrderId);
  expect(reviewed.status).toBe("reviewed");
  await expect(page.getByRole("status")).toContainText(`${reviewOrderNo} 已复核`);
  await page.screenshot({ path: path.join(artifactsDir, "outbound-review-submitted.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function openOutboundOrders(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M4 出库订单管理/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "出库业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "出库作业", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M4 出库订单管理" })).toBeVisible();
}

async function openOutboundWaves(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M4 波次规划/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "出库业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "出库作业", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M4 波次规划" })).toBeVisible();
}

async function openOutboundReview(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M4 复核发货/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "出库业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "出库作业", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M4 复核发货" })).toBeVisible();
}
