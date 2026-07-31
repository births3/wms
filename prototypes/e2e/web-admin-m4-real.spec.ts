import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { completeH9BusinessPrint } from "./h9-business-print";

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
  await page.reload();
  await expect(page.getByRole("heading", { name: "M4 出库订单管理" })).toBeVisible();
  await expect(page.getByText(created.wms_order_no, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-order-created.png"), fullPage: false });

  const createdRow = page.getByRole("row").filter({ hasText: created.wms_order_no }).first();
  await createdRow.getByRole("checkbox", { name: "选择此行" }).check();
  await completeH9BusinessPrint(page, {
    actionName: "打印",
    dialogName: "M4 随货同行单 E2E 模板",
    businessModule: "M4",
    templateType: "delivery_note",
    expectedField: "wms_order_no",
    expectedValue: created.wms_order_no,
    screenshotPath: path.join(artifactsDir, "delivery-note-preview.png"),
  });
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
    (response) => new URL(response.url()).pathname === "/api/v1/outbound/waves" && response.request().method() === "GET",
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
    (response) => new URL(response.url()).pathname === "/api/v1/outbound/waves" && response.request().method() === "GET",
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

  await page.setViewportSize({ width: 1440, height: 1000 });
  await row.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "交接", exact: true }).click();
  const handoverDialog = page.getByRole("dialog", { name: /发货交接/ });
  await expect(handoverDialog).toBeVisible();
  await handoverDialog.getByLabel("配送方类型").selectOption("own_fleet");
  await handoverDialog.getByLabel("车牌号").fill("沪A12345");
  await handoverDialog.getByLabel("车辆编号").fill("VEHICLE-E2E-001");
  await handoverDialog.getByLabel("司机用户 ID").fill("00000000-0000-4000-8000-000000000104");
  await expect(handoverDialog.getByLabel("签字附件 ID")).toBeVisible();
  await expect(handoverDialog.getByLabel("装车温度")).toBeVisible();
  await expect(handoverDialog.getByLabel("保温箱编号")).toBeVisible();
  await expect(handoverDialog.getByLabel("冰袋数量")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "outbound-handover-fields.png"), fullPage: false });
});

test("M4 查询条件命中默认窗口外的真实订单", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openOutboundOrders(page);

  const runTag = `M4-QUERY-${Date.now()}`;
  const targetOrderNo = `${runTag}-TARGET`;
  await createOutboundOrderViaHttp(page, targetOrderNo, `${runTag}-TARGET-ERP`);
  for (let index = 0; index < 50; index += 1) {
    await createOutboundOrderViaHttp(page, `${runTag}-FILLER-${index.toString().padStart(2, "0")}`, `${runTag}-FILLER-ERP-${index}`);
  }

  await page.getByLabel("关键字").fill(targetOrderNo);
  const queryResponsePromise = page.waitForResponse(
    (response) => response.url().includes("/api/v1/outbound/orders?")
      && response.url().includes("q=")
      && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "查询", exact: true }).click();
  const queryResponse = await queryResponsePromise;
  expect(queryResponse.ok()).toBeTruthy();
  await expect(page.getByText(targetOrderNo, { exact: true })).toBeVisible();
  await expect(page.getByRole("status")).toContainText("M4 出库订单管理已查询");
  await page.screenshot({ path: path.join(artifactsDir, "outbound-order-query-window.png"), fullPage: false });
});

// 【注意】M4 采购退货已从纯前端演示流程切换为真实接口（/api/v1/outbound/purchase-returns），
// 本文件（m4-real e2e）需要连真实后端重跑验证。
// 数据库可能没有采购退货种子数据，因此本用例先通过 UI 创建退货单，再走审批 → 拣货 → 复核 → 出库链路。
test("M4 PC 采购退货使用真实 API 走创建到出库全链路", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openOutboundReturns(page);

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "新建采购退货单" });
  await expect(createDialog).toBeVisible();
  const returnNo = `RTN-M4-E2E-${Date.now()}`;
  await fillPurchaseReturnForm(createDialog, returnNo);
  const createResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/purchase-returns") && response.request().method() === "POST",
  );
  await createDialog.getByRole("button", { name: "创建采购退货单" }).click();
  const createResponse = await createResponsePromise;
  const createBody = await createResponse.text();
  expect(createResponse.ok(), `M4 return create returned ${createResponse.status()}: ${createBody}`).toBeTruthy();
  const createdReturn = JSON.parse(createBody) as { id: string; return_no: string; status: string };
  expect(createdReturn.return_no).toBe(returnNo);
  expect(createdReturn.status).toBe("pending_approval");
  await expect(page.getByRole("status")).toContainText(`${createdReturn.return_no} 已创建`);
  await expect(page.getByText(createdReturn.return_no, { exact: true })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "purchase-return-created.png"), fullPage: false });

  const returnRow = page.getByRole("row").filter({ hasText: createdReturn.return_no }).first();
  await returnRow.getByRole("checkbox", { name: "选择此行" }).check();

  const chain = [
    { button: "审批", dialogName: /采购退货审批/, submit: "审批通过", endpoint: "approve", expectedStatus: "approved", toast: "采购退货审批已通过" },
    { button: "拣货", dialogName: /采购退货拣货/, submit: "确认拣货", endpoint: "pick", expectedStatus: "picking", toast: "采购退货拣货已完成" },
    { button: "复核", dialogName: /采购退货复核/, submit: "提交复核", endpoint: "review", expectedStatus: "reviewed", toast: "采购退货复核已完成" },
    { button: "出库", dialogName: /采购退货出库交接/, submit: "确认出库", endpoint: "ship", expectedStatus: "shipped", toast: "采购退货出库交接已完成" },
  ] as const;
  for (const step of chain) {
    await page.getByRole("button", { name: step.button, exact: true }).click();
    const actionDialog = page.getByRole("dialog", { name: step.dialogName });
    await expect(actionDialog).toBeVisible();
    const actionResponsePromise = page.waitForResponse(
      (response) =>
        response.url().endsWith(`/api/v1/outbound/purchase-returns/${createdReturn.id}/${step.endpoint}`)
        && response.request().method() === "POST",
    );
    await actionDialog.getByRole("button", { name: step.submit, exact: true }).click();
    const actionResponse = await actionResponsePromise;
    const actionBody = await actionResponse.text();
    expect(actionResponse.ok(), `M4 return ${step.endpoint} returned ${actionResponse.status()}: ${actionBody}`).toBeTruthy();
    const updatedReturn = JSON.parse(actionBody) as { id: string; status: string };
    expect(updatedReturn.id).toBe(createdReturn.id);
    expect(updatedReturn.status).toBe(step.expectedStatus);
    await expect(page.getByRole("status")).toContainText(step.toast);
  }
  await page.screenshot({ path: path.join(artifactsDir, "purchase-return-shipped.png"), fullPage: false });
});

// 驳回走真实接口且 reason 必填：新建一张退货单验证驳回分支。
test("M4 PC 采购退货驳回使用真实 API 提交必填原因", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  await login(page);
  await openOutboundReturns(page);

  await page.getByRole("button", { name: "新增", exact: true }).click();
  const createDialog = page.getByRole("dialog", { name: "新建采购退货单" });
  await fillPurchaseReturnForm(createDialog, `RTN-M4-REJECT-${Date.now()}`);
  const createResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/outbound/purchase-returns") && response.request().method() === "POST",
  );
  await createDialog.getByRole("button", { name: "创建采购退货单" }).click();
  const createResponse = await createResponsePromise;
  expect(createResponse.ok()).toBeTruthy();
  const createdReturn = JSON.parse(await createResponse.text()) as { id: string; return_no: string };

  const returnRow = page.getByRole("row").filter({ hasText: createdReturn.return_no }).first();
  await returnRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "驳回", exact: true }).click();
  const rejectDialog = page.getByRole("dialog", { name: /采购退货驳回/ });
  await expect(rejectDialog).toBeVisible();
  await rejectDialog.getByLabel("驳回备注（必填）").fill("E2E 驳回原因");
  const rejectResponsePromise = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/v1/outbound/purchase-returns/${createdReturn.id}/reject`)
      && response.request().method() === "POST",
  );
  await rejectDialog.getByRole("button", { name: "确认驳回", exact: true }).click();
  const rejectResponse = await rejectResponsePromise;
  const rejectBody = await rejectResponse.text();
  expect(rejectResponse.ok(), `M4 return reject returned ${rejectResponse.status()}: ${rejectBody}`).toBeTruthy();
  const rejectedReturn = JSON.parse(rejectBody) as { id: string; status: string; reject_reason?: string | null };
  expect(rejectedReturn.status).toBe("cancelled");
  await expect(page.getByRole("status")).toContainText("采购退货审批已驳回");
  await page.screenshot({ path: path.join(artifactsDir, "purchase-return-rejected.png"), fullPage: false });
});

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill("admin");
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function createOutboundOrderViaHttp(
  page: import("@playwright/test").Page,
  wmsOrderNo: string,
  erpOrderNo: string,
) {
  const accessToken = await page.evaluate(() => {
    const raw = window.localStorage.getItem("wms.web-admin.auth-session");
    return raw ? (JSON.parse(raw) as { accessToken?: string }).accessToken : null;
  });
  expect(accessToken).toBeTruthy();
  const apiBaseUrl = process.env.WMS_WEB_ADMIN_E2E_API_URL ?? "http://127.0.0.1:19185";
  const response = await page.request.post(`${apiBaseUrl}/api/v1/outbound/orders`, {
    headers: {
      authorization: `Bearer ${accessToken}`,
      "Idempotency-Key": `m4-query-${wmsOrderNo}`,
    },
    data: {
      document_type: "sales_outbound",
      wms_order_no: wmsOrderNo,
      erp_order_no: erpOrderNo,
      customer_id: "00000000-0000-0000-0000-000000001201",
      delivery_address_id: "00000000-0000-0000-0000-000000001211",
      warehouse_id: "00000000-0000-0000-0000-000000001301",
      required_ship_at: null,
      lines: [{ line_no: 1, product_code: "P-M1-E2E-001", batch_no: "B-M4-E2E-001", planned_qty: 1 }],
    },
  });
  const body = await response.text();
  expect(response.ok(), `M4 query seed returned ${response.status()}: ${body}`).toBeTruthy();
}

async function fillPurchaseReturnForm(
  dialog: import("@playwright/test").Locator,
  returnNo: string,
) {
  await dialog.getByLabel("采购退货单号").fill(returnNo);
  await dialog.getByLabel("原采购入库单").fill("ASN-M2-E2E-001");
  await dialog.getByLabel("供应商").fill("E2E 医药供应商");
  await expect(dialog.getByLabel("仓库")).not.toHaveValue("");
  await dialog.getByLabel("退货原因").fill("E2E 供应商召回");
  await dialog.getByLabel("商品编码").fill("P-M1-E2E-001");
  await dialog.getByLabel("数量").fill("3");
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

async function openOutboundReturns(page: import("@playwright/test").Page) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: /M4 采购退货出库/ });
  if (!(await target.isVisible())) {
    const section = navigation.getByRole("button", { name: "出库业务", exact: true });
    if ((await section.getAttribute("aria-expanded")) !== "true") await section.click();
    const group = navigation.getByRole("button", { name: "出库作业", exact: true });
    if ((await group.getAttribute("aria-expanded")) !== "true") await group.click();
  }
  await target.click();
  await expect(page.getByRole("heading", { name: "M4 采购退货出库" })).toBeVisible();
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
