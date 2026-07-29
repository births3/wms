import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../artifacts/screenshot-portal/real-web/m2-inbound");
const supplierId = "00000000-0000-0000-0000-000000001101";
const warehouseId = "00000000-0000-0000-0000-000000001301";
const adminUserId = "00000000-0000-0000-0000-000000000101";

test("M2 PC 真实入库链路落库并生成库存与审计", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });
  const receiptNo = `ASN-M2-E2E-${Date.now()}`;
  await login(page);
  const signerIds = await ensureReceivingClerks(page);

  await openMenu(page, "入库业务", "入库作业", /M2 收货管理/);
  await page.getByRole("button", { name: "新增", exact: true }).click();
  await page.getByLabel("ASN 号").fill(receiptNo);
  await page.getByLabel("单据类型", { exact: true }).selectOption("purchase_inbound");
  await page.getByLabel("供应商 ID").fill(supplierId);
  await page.getByLabel("仓库 ID").fill(warehouseId);
  await page.getByRole("dialog").getByLabel("预计到货").fill(localDateInputValue());
  await page.getByLabel("商品编码").fill("P-M1-E2E-001");
  await page.getByLabel("预报数量").fill("10");
  const createResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/inbound/receiving-orders") && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "创建 ASN" }).click();
  const createdOrder = await (await createResponsePromise).json() as { id: string };
  const receivingOrderId = createdOrder.id;
  await expect(page.getByText(`${receiptNo} 已创建`)).toBeVisible();

  await page.getByRole("button", { name: "放行", exact: true }).click();
  await expect(page.getByText(`${receiptNo} 已放行`)).toBeVisible();

  await page
    .locator("tbody tr")
    .filter({ hasText: receiptNo })
    .getByRole("checkbox", { name: "选择此行" })
    .check();
  await page.getByRole("button", { name: "打印", exact: true }).click();
  const asnPrintDialog = page.getByRole("dialog", { name: "M2 ASN E2E 模板" });
  await expect(asnPrintDialog).toBeVisible();
  await expect(asnPrintDialog).toHaveCSS("opacity", "1");
  await expect(asnPrintDialog.getByText("ASN 号")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "receiving-print-preview.png") });
  await asnPrintDialog.getByRole("button", { name: "打印", exact: true }).click();
  await expect(asnPrintDialog.getByText("确认打印结果")).toBeVisible();
  await asnPrintDialog.screenshot({ path: path.join(artifactsDir, "browser-print-result-confirmation.png") });
  const asnPrintResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/print-templates/print") && response.request().method() === "POST",
  );
  await asnPrintDialog.getByRole("button", { name: "已完成打印", exact: true }).click();
  const asnPrintResponse = await asnPrintResponsePromise;
  const asnPrintBody = JSON.parse(asnPrintResponse.request().postData() ?? "{}") as Record<string, unknown>;
  expect(asnPrintBody.business_module).toBe("M2");
  expect(asnPrintBody.business_document_type).toBe("asn");
  expect(asnPrintBody.business_document_id).toBe(receivingOrderId);
  expect(asnPrintBody.status).toBe("printed");
  expect((asnPrintBody.data as { order?: { receipt_no?: string } }).order?.receipt_no).toBe(receiptNo);
  await expect(asnPrintResponse.json()).resolves.toMatchObject({ retry_count: 0, status: "printed" });
  await expect(page.getByRole("status").filter({ hasText: "打印记录已写入" })).toBeVisible();

  for (const [resultLabel, expectedStatus, expectedRetry] of [
    ["已取消", "cancelled", 1],
    ["打印失败", "failed", 2],
  ] as const) {
    await page.getByRole("button", { name: "打印", exact: true }).click();
    const retryDialog = page.getByRole("dialog", { name: "M2 ASN E2E 模板" });
    await retryDialog.getByRole("button", { name: "打印", exact: true }).click();
    await expect(retryDialog.getByText("确认打印结果")).toBeVisible();
    const retryResponsePromise = page.waitForResponse(
      (response) => response.url().endsWith("/api/v1/print-templates/print") && response.request().method() === "POST",
    );
    await retryDialog.getByRole("button", { name: resultLabel, exact: true }).click();
    const retryResponse = await retryResponsePromise;
    const retryBody = JSON.parse(retryResponse.request().postData() ?? "{}") as Record<string, unknown>;
    expect(retryBody.business_document_id).toBe(receivingOrderId);
    expect(retryBody.status).toBe(expectedStatus);
    expect((retryBody.data as { order?: { receipt_no?: string } }).order?.receipt_no).toBe(receiptNo);
    if (expectedStatus === "failed") expect(retryBody.failure_reason).toBe("用户确认浏览器打印失败");
    await expect(retryResponse.json()).resolves.toMatchObject({
      retry_count: expectedRetry,
      status: expectedStatus,
    });
  }

  await page.getByRole("button", { name: "收货", exact: true }).click();
  await page.getByLabel("实际到货数量").fill("10");
  await page.getByLabel("缺货数量").fill("0");
  await page.getByLabel("拒收数量", { exact: true }).fill("0");
  await expect(page.getByLabel("商品温度属性")).toHaveValue("冷藏");
  await page.getByLabel("到货温度 (°C)").fill("5");
  await page.getByLabel("冷链运输方式").fill("冷藏车");
  await page.getByRole("button", { name: "确认收货" }).click();
  await expect(page.getByText(`${receiptNo} 收货已提交`)).toBeVisible();
  const printDataResponse = await page.evaluate(async (id) => {
    const session = JSON.parse(window.localStorage.getItem("wms.web-admin.auth-session") ?? "null") as { accessToken?: string } | null;
    const response = await fetch(`/api/v1/inbound/receiving-orders/${id}/print-data`, {
      headers: session?.accessToken ? { Authorization: `Bearer ${session.accessToken}` } : undefined,
    });
    return { status: response.status, body: await response.json() };
  }, receivingOrderId);
  expect(printDataResponse.status).toBe(200);
  expect(printDataResponse.body).toMatchObject({
    order: { id: receivingOrderId },
    receipts: [{ actual_qty: 10, shortage_qty: 0, rejected_qty: 0 }],
  });
  expect(printDataResponse.body.receipts[0].details).toMatchObject({
    vehicle_no: "沪A-12345",
    carrier: "华东冷链承运商",
    origin: "上海配送中心",
  });
  await page.screenshot({ path: path.join(artifactsDir, "receiving.png") });

  await page.getByRole("button", { name: "进度看板" }).click();
  await expect(page.getByRole("heading", { name: "M2 入库进度看板" })).toBeVisible();
  await expect(page.getByRole("combobox", { name: "刷新间隔" })).toHaveValue("30");
  await page.getByRole("textbox", { name: "供应商 ID" }).fill(supplierId);
  await page.getByRole("textbox", { name: "商品编码" }).fill("P-M1-E2E-001");
  await page.getByRole("button", { name: "查询" }).click();
  const inspectingRow = page.locator("table tbody tr").filter({ hasText: "验收中" });
  await expect(inspectingRow).toBeVisible();
  await inspectingRow.click();
  await expect(page.getByRole("dialog", { name: "状态单据" })).toBeVisible();
  await page.getByRole("dialog", { name: "状态单据" }).getByRole("button", { name: new RegExp(receiptNo) }).click();
  await expect(page.getByRole("dialog", { name: "订单详情" })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "receiving-dashboard-detail.png") });
  await page.keyboard.press("Escape");

  await openMenu(page, "入库业务", "入库作业", /M2 验收管理/);
  await expect(page.locator("table").getByText(receiptNo, { exact: true })).toBeVisible();

  await page
    .locator("tbody tr")
    .filter({ hasText: receiptNo })
    .getByRole("checkbox", { name: "选择此行" })
    .check();
  await page.getByRole("button", { name: "打印", exact: true }).click();
  const acceptancePrintDialog = page.getByRole("dialog", { name: "M2 验收记录 E2E 模板" });
  await expect(acceptancePrintDialog).toBeVisible();
  await expect(acceptancePrintDialog).toHaveCSS("opacity", "1");
  await expect(acceptancePrintDialog.getByText("ASN 号")).toBeVisible();
  await expect(acceptancePrintDialog.getByText(receiptNo)).toBeVisible();
  await acceptancePrintDialog.screenshot({ path: path.join(artifactsDir, "inspection-print-preview.png") });
  await acceptancePrintDialog.getByRole("button", { name: "打印", exact: true }).click();
  await expect(acceptancePrintDialog.getByText("确认打印结果")).toBeVisible();
  const acceptancePrintResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith("/api/v1/print-templates/print") && response.request().method() === "POST",
  );
  await acceptancePrintDialog.getByRole("button", { name: "已完成打印", exact: true }).click();
  const acceptancePrintResponse = await acceptancePrintResponsePromise;
  const acceptancePrintBody = JSON.parse(acceptancePrintResponse.request().postData() ?? "{}") as Record<string, unknown>;
  expect(acceptancePrintBody.business_module).toBe("M2");
  expect(acceptancePrintBody.business_document_type).toBe("acceptance_record");
  expect(acceptancePrintBody.business_document_id).toBe(receivingOrderId);
  expect(acceptancePrintBody.status).toBe("printed");
  expect((acceptancePrintBody.data as { receipts?: Array<{ actual_qty?: number }> }).receipts?.at(-1)?.actual_qty).toBe(10);
  await expect(acceptancePrintResponse.json()).resolves.toMatchObject({ retry_count: 0, status: "printed" });
  await expect(page.getByRole("status").filter({ hasText: "打印记录已写入" })).toBeVisible();

  const overActualInspection = await page.evaluate(async (id) => {
    const session = JSON.parse(window.localStorage.getItem("wms.web-admin.auth-session") ?? "null") as { accessToken?: string } | null;
    const response = await fetch(`/api/v1/inbound/receiving-orders/${id}/inspect`, {
      method: "POST",
      headers: {
        Authorization: session?.accessToken ? `Bearer ${session.accessToken}` : "",
        "Content-Type": "application/json",
        "Idempotency-Key": `m2-e2e-over-actual-${Date.now()}`,
      },
      body: JSON.stringify({
        batch_no: "B-M2-E2E-OVER",
        accepted_qty: 11,
        rejected_qty: 0,
        production_date: "2026-01-01",
        expiry_date: "2028-01-01",
        quality_status: "qualified",
        trace_codes: ["TRACE-M2-E2E-OVER"],
      }),
    });
    return { status: response.status, body: await response.json() };
  }, receivingOrderId);
  expect(overActualInspection.status).toBe(422);
  expect(overActualInspection.body).toMatchObject({ severity: "error" });

  const inspectionRequests: import("@playwright/test").Request[] = [];
  const signatureRequests: import("@playwright/test").Request[] = [];
  const trackAcceptanceRequests = (request: import("@playwright/test").Request) => {
    const pathname = new URL(request.url()).pathname;
    if (request.method() !== "POST") return;
    if (pathname.endsWith(`/api/v1/inbound/receiving-orders/${receivingOrderId}/inspect`)) inspectionRequests.push(request);
    if (pathname.endsWith(`/api/v1/inbound/receiving-orders/${receivingOrderId}/sign`)) signatureRequests.push(request);
  };
  page.on("request", trackAcceptanceRequests);
  const policyResponsePromise = page.waitForResponse(
    (response) => response.url().includes("/api/v1/m-vr/dual-person-policy?") && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "验收", exact: true }).click();
  const policyResponse = await policyResponsePromise;
  expect(policyResponse.ok()).toBeTruthy();
  expect(await policyResponse.json()).toMatchObject({ policy: "dual_scan", process: "入库", node: "验收" });
  await expect(page.getByText(/M-VR：双人扫码/)).toBeVisible();
  await page.getByLabel("验收批号").fill("B-M2-E2E-001");
  await page.getByLabel("通过数量").fill("10");
  await page.getByLabel("拒收数量", { exact: true }).fill("0");
  await page.getByLabel("生产日期").fill("2026-01-01");
  await page.getByLabel("有效期至").fill("2028-01-01");
  await page.getByLabel("追溯码").fill("TRACE-M2-E2E-001");
  await page.getByRole("dialog", { name: "验收" }).getByRole("combobox", { name: "质量状态" }).selectOption("qualified");
  for (const label of ["外观核对", "包装核对", "说明书核对", "标签核对"]) await page.getByLabel(label).fill("通过");
  // 第一人必须是当前登录用户；禁止一次提交两名签字人代签。
  await page.screenshot({ path: path.join(artifactsDir, "inspection-dual-sign-validation.png") });
  const inspectionResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/inbound/receiving-orders/${receivingOrderId}/inspect`) && response.request().method() === "POST",
  );
  const firstSignatureResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/inbound/receiving-orders/${receivingOrderId}/sign`) && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "提交验收" }).click();
  const inspectionResponse = await inspectionResponsePromise;
  const firstSignatureResponse = await firstSignatureResponsePromise;
  expect(inspectionResponse.status()).toBe(200);
  expect(firstSignatureResponse.status()).toBe(200);
  expect(JSON.parse(signatureRequests[0]?.postData() ?? "{}")).toMatchObject({
    first_signer_id: adminUserId,
    second_signer_id: null,
    dual_required: true,
  });
  expect(await firstSignatureResponse.json()).toMatchObject({
    receiving_order_id: receivingOrderId,
    first_signer_id: adminUserId,
    second_signer_id: null,
  });
  await expect(page.getByText(`${receiptNo} 第一人已签字，待第二人独立登录签字`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inspection.png") });
  page.off("request", trackAcceptanceRequests);

  // 第二人独立登录并完成签字。
  await page.evaluate(() => window.localStorage.clear());
  await loginAs(page, "m2-e2e-receiving-clerk");
  await openMenu(page, "入库业务", "入库作业", /M2 验收管理/);
  await page.locator("table").getByText(receiptNo, { exact: true }).click();
  const secondSignatureResponsePromise = page.waitForResponse(
    (response) => response.url().endsWith(`/api/v1/inbound/receiving-orders/${receivingOrderId}/sign`) && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "第二签字", exact: true }).click();
  await page.getByRole("button", { name: "提交验收" }).click();
  const secondSignatureResponse = await secondSignatureResponsePromise;
  expect(secondSignatureResponse.status()).toBe(200);
  expect(await secondSignatureResponse.json()).toMatchObject({
    receiving_order_id: receivingOrderId,
    first_signer_id: adminUserId,
    second_signer_id: signerIds.secondSignerId,
  });
  await expect(page.getByText(`${receiptNo} 第二人签字已完成`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inspection-dual-sign-submitted.png") });

  await page.evaluate(() => window.localStorage.clear());
  await login(page);

  await openMenu(page, "入库业务", "入库作业", /M2 上架管理/);
  await expect(page.locator("table").getByText(receiptNo, { exact: true })).toBeVisible();
  const recommendationResponsePromise = page.waitForResponse(
    (response) => response.url().includes(`/api/v1/inbound/receiving-orders/${receivingOrderId}/putaway-recommendations`) && response.request().method() === "GET",
  );
  await page.getByRole("button", { name: "上架", exact: true }).click();
  const putawayDialog = page.getByRole("dialog", { name: "上架" });
  await expect(putawayDialog).toBeVisible();
  await putawayDialog.getByLabel("数量", { exact: true }).fill("0");
  await putawayDialog.getByRole("button", { name: "确认上架" }).click();
  await expect(putawayDialog.getByRole("alert").first()).toContainText("上架数量必须大于 0");
  await putawayDialog.getByLabel("数量", { exact: true }).fill("10");
  const recommendationResponse = await recommendationResponsePromise;
  expect(recommendationResponse.status()).toBe(200);
  const recommendationBody = await recommendationResponse.json() as { data: Array<{ location_code: string; same_product: boolean }> };
  expect(recommendationBody.data.length).toBeGreaterThan(0);
  await expect(putawayDialog.getByText("推荐 #1", { exact: false })).toBeVisible();
  await expect(putawayDialog.getByText("推荐原因：", { exact: false })).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "putaway-recommendations.png") });
  await putawayDialog.locator('input[name="putaway-recommended-location"]').first().check();
  await page.getByLabel("上架商品编码").fill("P-M1-E2E-001");
  await page.getByLabel("上架批号").fill("B-M2-E2E-001");
  await page.getByLabel("数量", { exact: true }).fill("10");
  await page.getByLabel("实际库位").fill("A01-01-02-03");
  await page.getByRole("button", { name: "确认上架" }).click();
  await expect(page.getByText(`${receiptNo} 上架已提交`)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "putaway.png") });

  await openMenu(page, "入库业务", "入库作业", /M2 上架策略/);
  await expect(page.getByTestId("m2-putaway-strategy-page")).toBeVisible();
  await page.getByRole("button", { name: "新增", exact: true }).click();
  const strategyDialog = page.getByTestId("m2-putaway-strategy-dialog");
  await expect(strategyDialog).toBeVisible();
  await strategyDialog.getByLabel("方案编码").fill(`e2e-${Date.now()}`);
  await strategyDialog.getByLabel("方案名称").fill("E2E 策略方案");
  await strategyDialog.getByLabel("Top N").fill("3");
  await expect(strategyDialog.getByTestId("m2-putaway-rule-priority")).toBeVisible();
  await expect(strategyDialog.getByText("温区匹配")).toBeVisible();
  const strategySavePromise = page.waitForResponse(
    (response) =>
      response.url().includes("/api/v1/inbound/putaway-strategy-profiles") &&
      response.request().method() === "PUT",
  );
  await strategyDialog.getByRole("button", { name: "保存" }).scrollIntoViewIfNeeded();
  await strategyDialog.getByRole("button", { name: "保存" }).click({ force: true });
  const strategySaveResponse = await strategySavePromise;
  expect(strategySaveResponse.status()).toBe(200);
  await expect(page.getByText(/方案 .* 已保存/)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "putaway-strategy-config.png") });

  await openMenu(page, "库内业务", "库存管理", /M3 批号管理/);
  await expect(page.getByText("B-M2-E2E-001").first()).toBeVisible();
  const inventoryRow = page.getByRole("row").filter({ hasText: "B-M2-E2E-001" }).first();
  await inventoryRow.getByRole("checkbox", { name: "选择此行" }).check();
  await page.getByRole("button", { name: "详情", description: "查看选中批号详情", exact: true }).click();
  const traceDialog = page.getByRole("dialog", { name: "批号详情" });
  await expect(traceDialog).toBeVisible();
  await expect(traceDialog.getByText(/库存 movement：\d+ 条/)).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory-trace.png") });
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "状态", exact: true }).click();
  const statusDialog = page.getByRole("dialog", { name: "变更库存状态" });
  await expect(statusDialog).toBeVisible();
  await statusDialog.getByLabel("目标状态").selectOption("quarantined");
  await statusDialog.getByLabel("审批来源").fill("温度超标事件");
  await statusDialog.getByLabel("审批编号").fill("TEMP-M2-E2E-001");
  await statusDialog.getByLabel("变更原因").fill("真实 E2E 状态变更");
  await statusDialog.getByRole("button", { name: "确认变更" }).click();
  await expect(page.getByRole("status")).toContainText("状态已更新");
  await expect(inventoryRow.getByText("隔离")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory.png") });

  await page.locator('button[title="标记选中库存批次召回并隔离"]').click();
  const recallDialog = page.getByRole("dialog", { name: "标记召回" });
  await expect(recallDialog).toBeVisible();
  await recallDialog.getByLabel("审批编号").fill(`RECALL-M2-E2E-${Date.now()}`);
  await recallDialog.getByLabel("召回原因").fill("真实 E2E 召回隔离");
  await recallDialog.getByRole("button", { name: "确认召回" }).click();
  await expect(page.getByRole("status")).toContainText("已标记召回");
  await expect(inventoryRow.getByText("已标记")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory-recall.png") });

  await page.locator('button[title="双人审批取消选中库存批次召回"]').click();
  const cancelRecallDialog = page.getByRole("dialog", { name: "取消召回" });
  await expect(cancelRecallDialog).toBeVisible();
  await cancelRecallDialog.getByLabel("取消审批编号").fill(`RECALL-CANCEL-M2-E2E-${Date.now()}`);
  await cancelRecallDialog.getByLabel("质量审批人 ID").fill("00000000-0000-0000-0000-000000000201");
  await cancelRecallDialog.getByLabel("取消原因").fill("真实 E2E 质量复核后取消召回");
  await cancelRecallDialog.getByRole("button", { name: "确认取消" }).click();
  await expect(page.getByRole("status")).toContainText("已取消召回");
  await expect(inventoryRow.getByText("未标记")).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "inventory-recall-cancel.png") });
});

async function login(page: import("@playwright/test").Page) {
  await loginAs(page, "admin");
}

async function loginAs(page: import("@playwright/test").Page, username: string) {
  await page.goto("/");
  await page.getByLabel("货主编码").fill("PY_OWNER");
  await page.getByLabel("登录账号").fill(username);
  await page.getByRole("textbox", { name: "密码", exact: true }).fill("CorrectHorse1!");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();
}

async function ensureReceivingClerks(page: import("@playwright/test").Page) {
  return page.evaluate(async ({ adminUserId }) => {
    const session = JSON.parse(window.localStorage.getItem("wms.web-admin.auth-session") ?? "null") as { accessToken?: string } | null;
    const headers = { Authorization: `Bearer ${session?.accessToken ?? ""}`, "Content-Type": "application/json" };
    async function request(path: string, init: RequestInit = {}) {
      const response = await fetch(path, { ...init, headers: { ...headers, ...(init.headers ?? {}) } });
      if (!response.ok) throw new Error(`E2E 角色准备失败: ${path} ${response.status} ${await response.text()}`);
      return response.json() as Promise<unknown>;
    }
    const roles = await request("/api/v1/auth/roles") as { items: Array<{ id: string; role_code: string }> };
    let receivingRole = roles.items.find((role) => role.role_code === "receiving_clerk");
    if (!receivingRole) {
      receivingRole = await request("/api/v1/auth/roles", {
        method: "POST",
        headers: { ...headers, "Idempotency-Key": "m2-e2e-create-receiving-clerk" },
        body: JSON.stringify({ role_code: "receiving_clerk", role_name: "收货员", data_scope: "warehouse", parent_role_id: null }),
      }) as { id: string; role_code: string };
    }
    const users = await request("/api/v1/auth/users") as { items: Array<{ user_id: string; username: string }> };
    let firstSigner = users.items.find((user) => user.username === "m2-e2e-receiving-clerk-first");
    if (!firstSigner) {
      firstSigner = await request("/api/v1/auth/users", {
        method: "POST",
        headers: { ...headers, "Idempotency-Key": "m2-e2e-create-first-receiving-user" },
        body: JSON.stringify({ username: "m2-e2e-receiving-clerk-first", display_name: "M2 E2E 第一收货员", phone: "13900000001", password: "CorrectHorse1!", role_ids: [receivingRole.id] }),
      }) as { user_id: string; username: string };
    }
    let secondSigner = users.items.find((user) => user.username === "m2-e2e-receiving-clerk");
    if (!secondSigner) {
      secondSigner = await request("/api/v1/auth/users", {
        method: "POST",
        headers: { ...headers, "Idempotency-Key": "m2-e2e-create-receiving-user" },
        body: JSON.stringify({ username: "m2-e2e-receiving-clerk", display_name: "M2 E2E 收货员", phone: "13800000001", password: "CorrectHorse1!", role_ids: [receivingRole.id] }),
      }) as { user_id: string; username: string };
    }
    const systemRole = roles.items.find((role) => role.role_code === "system_admin");
    if (!systemRole) throw new Error("E2E 角色准备失败: 缺少 system_admin");
    await request("/api/v1/auth/user-roles/batch", {
      method: "PUT",
      headers: { ...headers, "Idempotency-Key": "m2-bind-users" },
      body: JSON.stringify({ user_ids: [adminUserId, firstSigner.user_id, secondSigner.user_id], role_ids: [systemRole.id, receivingRole.id] }),
    });
    return { firstSignerId: firstSigner.user_id, secondSignerId: secondSigner.user_id };
  }, { adminUserId });
}

async function openMenu(page: import("@playwright/test").Page, section: string, group: string, item: RegExp) {
  const navigation = page.getByRole("navigation");
  const target = navigation.getByRole("button", { name: item });
  if (!(await target.isVisible())) {
    const sectionButton = navigation.getByRole("button", { name: section, exact: true });
    if ((await sectionButton.getAttribute("aria-expanded")) !== "true") await sectionButton.click();
    const groupButton = navigation.getByRole("button", { name: group, exact: true });
    if ((await groupButton.getAttribute("aria-expanded")) !== "true") await groupButton.click();
  }
  await target.click();
}

function localDateInputValue(value = new Date()) {
  const month = String(value.getMonth() + 1).padStart(2, "0");
  const day = String(value.getDate()).padStart(2, "0");
  return `${value.getFullYear()}-${month}-${day}`;
}
