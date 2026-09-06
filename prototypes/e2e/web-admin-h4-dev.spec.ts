import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

const artifactsDir = path.resolve("../apps/web-admin/.e2e-artifacts/h4-dev/screenshots");

test("H4 管理端菜单能打开通知配置和发送记录", async ({ page }) => {
  fs.mkdirSync(artifactsDir, { recursive: true });

  await page.goto("/");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByRole("heading", { name: "运营总览" })).toBeVisible();

  await page.getByRole("button", { name: "基础能力" }).click();
  await page.getByRole("button", { name: "H4 企业微信" }).click();
  await page.getByRole("button", { name: /H4 参数设置/ }).click();
  await expect(page.getByRole("heading", { name: "H4 参数设置" })).toBeVisible();
  await expect(page.getByText("ww-demo-corp")).toBeVisible();
  const settingsPage = page.getByRole("heading", { name: "H4 参数设置" }).locator("..");
  await settingsPage.getByRole("row").filter({ hasText: "ww-demo-corp" }).getByRole("checkbox").check();
  await settingsPage.getByRole("button", { name: "修改" }).click();
  const settingsDialog = page.getByRole("dialog", { name: "企业微信参数设置" });
  await expect(settingsDialog).toBeVisible();
  await settingsDialog.getByLabel("重试间隔秒").fill("75");
  const requestOrder: string[] = [];
  const trackSettingsRequests = (request: import("@playwright/test").Request) => {
    if (request.method() !== "POST") return;
    if (request.url().includes("/api/v1/wechat-notify/settings/test")) requestOrder.push("test-request");
    else if (request.url().includes("/api/v1/wechat-notify/settings")) requestOrder.push("save-request");
  };
  const trackSettingsResponses = (response: import("@playwright/test").Response) => {
    if (response.request().method() === "POST"
      && response.url().includes("/api/v1/wechat-notify/settings")
      && !response.url().includes("/test")) {
      requestOrder.push("save-response");
    }
  };
  page.on("request", trackSettingsRequests);
  page.on("response", trackSettingsResponses);
  const saveResponse = page.waitForResponse((response) =>
    response.url().includes("/api/v1/wechat-notify/settings")
      && !response.url().includes("/test")
      && response.request().method() === "POST",
  );
  const testResponse = page.waitForResponse((response) =>
    response.url().includes("/api/v1/wechat-notify/settings/test") && response.request().method() === "POST",
  );
  await settingsDialog.getByRole("button", { name: "测试" }).click();
  const savedResponse = await saveResponse;
  expect(savedResponse.status()).toBe(200);
  expect(savedResponse.request().postDataJSON()).toMatchObject({ retry_interval_seconds: 75 });
  const response = await testResponse;
  page.off("request", trackSettingsRequests);
  page.off("response", trackSettingsResponses);
  expect(requestOrder).toEqual(["save-request", "save-response", "test-request"]);
  expect(response.status()).toBe(200);
  await expect(response.json()).resolves.toMatchObject({ status: "success" });
  await expect(page.getByText("企业微信参数校验通过")).toBeVisible();

  const invalidSettingsResponse = await page.request.post("/api/v1/wechat-notify/settings", {
    headers: { "Idempotency-Key": "h4-e2e-invalid-settings" },
    data: {},
  });
  expect(invalidSettingsResponse.status()).toBe(422);
  await expect(invalidSettingsResponse.json()).resolves.toMatchObject({
    code: "H4_REQUEST_INVALID",
    severity: "error",
    details: {},
    trace_id: "dev-mock",
  });
  const invalidConfigResponse = await page.request.post("/api/v1/wechat-notify/configs", {
    headers: { "Idempotency-Key": "h4-e2e-invalid-config" },
    data: {
      event_type: "",
      enabled: true,
      template: "",
      recipient_rule: {},
      channels: ["wechat"],
    },
  });
  expect(invalidConfigResponse.status()).toBe(422);
  await expect(invalidConfigResponse.json()).resolves.toMatchObject({ code: "H4_REQUEST_INVALID" });

  const invalidUrlSettingsResponse = await page.request.post("/api/v1/wechat-notify/settings", {
    headers: { "Idempotency-Key": "test-url" },
    data: {
      corp_id: "ww-demo-corp",
      agent_id: "1000002",
      secret_alias: "h4/wechat/secret",
      callback_token_alias: "h4/wechat/token",
      aes_key_alias: "h4/wechat/aes",
      callback_url: "not-a-url",
      approval_callback_path: "callback-without-leading-slash",
      enabled: true,
      retry_max_attempts: 3,
      retry_interval_seconds: 60,
    },
  });
  expect(invalidUrlSettingsResponse.status()).toBe(422);
  await expect(invalidUrlSettingsResponse.json()).resolves.toMatchObject({ code: "H4_REQUEST_INVALID" });

  const invalidApprovalResponse = await page.request.post("/api/v1/wechat-notify/approvals", {
    headers: { "Idempotency-Key": "h4-e2e-invalid-approval-path" },
    data: {
      scenario: "asn_cancel",
      business_ref: "ASN-H4-E2E",
      dedupe_key: "test-approval",
      approver_user: "00000000-0000-0000-0000-000000000101",
      process_id: "ww-process-e2e",
      callback_path: "https://evil.example/callback",
      summary: "非法回调路径",
    },
  });
  expect(invalidApprovalResponse.status()).toBe(422);
  await expect(invalidApprovalResponse.json()).resolves.toMatchObject({ code: "H4_REQUEST_INVALID" });
  const approvalResponse = await page.request.post("/api/v1/wechat-notify/approvals", {
    headers: { "Idempotency-Key": "h4-e2e-approval" },
    data: {
      scenario: "asn_cancel",
      business_ref: "ASN-H4-E2E",
      dedupe_key: "test-approval",
      approver_user: "00000000-0000-0000-0000-000000000101",
      process_id: "ww-process-e2e",
      callback_path: "/api/v1/wechat-notify/approvals/{approval_id}/callback",
      summary: "ASN 作废审批",
    },
  });
  const approval = await approvalResponse.json() as { id: string; status: string };
  expect(approval.status).toBe("pending");
  const callbackResponse = await page.request.post(
    `/api/v1/wechat-notify/approvals/${approval.id}/callback`,
    {
      headers: { "Idempotency-Key": "h4-e2e-approval-callback" },
      data: {
        conclusion: "approved",
        opinion: "同意",
        approved_by: "00000000-0000-0000-0000-000000000101",
        external_approval_id: "ww-approval-e2e",
      },
    },
  );
  expect(callbackResponse.status()).toBe(200);
  await expect(callbackResponse.json()).resolves.toMatchObject({ status: "approved" });

  const invalidDateResponse = await page.request.get("/api/v1/wechat-notify/records?from=not-a-date");
  expect(invalidDateResponse.status()).toBe(400);
  await expect(invalidDateResponse.json()).resolves.toMatchObject({ code: "DEV_MOCK_REQUEST_INVALID" });

  const missingIdempotencyResponse = await page.request.post("/api/v1/wechat-notify/configs", {
    data: {
      event_type: "asn_arrived",
      enabled: true,
      template: "ASN {{asn_no}} 已到货",
      recipient_rule: { users: ["receiving_lead"] },
      channels: ["wechat"],
    },
  });
  expect(missingIdempotencyResponse.status()).toBe(400);
  await expect(missingIdempotencyResponse.json()).resolves.toMatchObject({ code: "H4_IDEMPOTENCY_REQUIRED" });

  const idempotentConfig = {
    event_type: "h4_e2e_idempotency",
    enabled: true,
    template: "{{message}}",
    recipient_rule: { users: ["receiving_lead"] },
    channels: ["wechat"],
  };
  const firstConfigResponse = await page.request.post("/api/v1/wechat-notify/configs", {
    headers: { "Idempotency-Key": "h4-e2e-config-replay" },
    data: idempotentConfig,
  });
  const firstConfig = await firstConfigResponse.json() as { id: string; version: number };
  const replayConfigResponse = await page.request.post("/api/v1/wechat-notify/configs", {
    headers: { "Idempotency-Key": "h4-e2e-config-replay" },
    data: idempotentConfig,
  });
  await expect(replayConfigResponse.json()).resolves.toMatchObject(firstConfig);
  const conflictingConfigResponse = await page.request.post("/api/v1/wechat-notify/configs", {
    headers: { "Idempotency-Key": "h4-e2e-config-replay" },
    data: { ...idempotentConfig, template: "冲突 {{message}}" },
  });
  expect(conflictingConfigResponse.status()).toBe(409);
  await expect(conflictingConfigResponse.json()).resolves.toMatchObject({ code: "H4_IDEMPOTENCY_CONFLICT" });

  const dedupeKey = `h4-e2e-${Date.now()}`;
  const sendResponse = await page.request.post("/api/v1/wechat-notify/send", {
    headers: { "Idempotency-Key": "h4-e2e-send" },
    data: {
      event_type: "asn_arrived",
      recipients: ["receiving_lead"],
      dedupe_key: dedupeKey,
      payload: { asn_no: "ASN-H4-E2E" },
    },
  });
  expect(sendResponse.status()).toBe(200);
  const sentRecords = await sendResponse.json() as Array<{ id: string; dedupe_key: string; status: string }>;
  expect(sentRecords).toMatchObject([{ dedupe_key: dedupeKey, status: "failed" }]);
  const duplicateSendResponse = await page.request.post("/api/v1/wechat-notify/send", {
    headers: { "Idempotency-Key": "h4-e2e-send-duplicate" },
    data: {
      event_type: "asn_arrived",
      recipients: ["receiving_lead"],
      dedupe_key: dedupeKey,
      payload: { asn_no: "ASN-H4-E2E" },
    },
  });
  expect(duplicateSendResponse.status()).toBe(200);
  await expect(duplicateSendResponse.json()).resolves.toMatchObject([{ id: sentRecords[0].id }]);
  const invalidSendResponse = await page.request.post("/api/v1/wechat-notify/send", {
    headers: { "Idempotency-Key": "test-empty" },
    data: { event_type: "asn_arrived", recipients: [], dedupe_key: "test-empty", payload: { asn_no: "ASN-H4-E2E" } },
  });
  expect(invalidSendResponse.status()).toBe(422);
  await expect(invalidSendResponse.json()).resolves.toMatchObject({ code: "H4_NO_RECIPIENTS" });
  const missingVariableResponse = await page.request.post("/api/v1/wechat-notify/send", {
    headers: { "Idempotency-Key": "test-missing" },
    data: { event_type: "asn_arrived", recipients: ["receiving_lead"], dedupe_key: "test-missing", payload: {} },
  });
  expect(missingVariableResponse.status()).toBe(422);
  await expect(missingVariableResponse.json()).resolves.toMatchObject({ code: "H4_TEMPLATE_INVALID" });
  const invalidPayloadResponse = await page.request.post("/api/v1/wechat-notify/send", {
    headers: { "Idempotency-Key": "h4-e2e-invalid-payload" },
    data: { event_type: "asn_arrived", recipients: ["receiving_lead"], dedupe_key: "h4-invalid-payload", payload: null },
  });
  expect(invalidPayloadResponse.status()).toBe(422);
  await expect(invalidPayloadResponse.json()).resolves.toMatchObject({ code: "H4_TEMPLATE_INVALID" });
  const malformedJsonResponse = await page.evaluate(async () => {
    const response = await fetch("/api/v1/wechat-notify/send", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Idempotency-Key": "h4-e2e-malformed-json",
      },
      body: "{",
    });
    return { status: response.status, body: await response.json() };
  });
  expect(malformedJsonResponse.status).toBe(400);
  expect(malformedJsonResponse.body).toMatchObject({ code: "DEV_MOCK_REQUEST_INVALID" });
  const recordsResponse = await page.request.get(
    "/api/v1/wechat-notify/records?event_type=asn_arrived&recipient=receiving_lead",
  );
  const records = await recordsResponse.json() as { data: Array<{ dedupe_key: string }> };
  expect(records.data.some((record) => record.dedupe_key === dedupeKey)).toBe(true);
  const localDayResponse = await page.request.get(
    "/api/v1/wechat-notify/records?from=2026-07-08T16%3A00%3A00.000Z&to=2026-07-09T15%3A59%3A59.999Z",
  );
  const localDayRecords = await localDayResponse.json() as { data: Array<{ id: string }> };
  expect(localDayRecords.data).toEqual([]);

  // 该拦截只验证前端错误展示；上面的请求验证 dev mock 的真实 422 链路。
  await page.route("**/api/v1/wechat-notify/settings/test", async (route) => {
    await route.fulfill({
      status: 422,
      contentType: "application/json",
      body: JSON.stringify({
        code: "H4_REQUEST_INVALID",
        message: "企业微信参数不完整",
        severity: "error",
        details: {},
        trace_id: "e2e",
      }),
    });
  });
  await settingsPage.getByRole("button", { name: "修改" }).click();
  await settingsDialog.getByRole("button", { name: "测试" }).click();
  await expect(settingsDialog).toBeHidden();
  await expect(page.getByText(/企业微信参数不完整/)).toBeVisible();
  await page.unroute("**/api/v1/wechat-notify/settings/test");

  let testCalledAfterSaveFailure = false;
  const trackBlockedTestRequest = (request: import("@playwright/test").Request) => {
    if (request.method() === "POST" && request.url().includes("/api/v1/wechat-notify/settings/test")) {
      testCalledAfterSaveFailure = true;
    }
  };
  page.on("request", trackBlockedTestRequest);
  await page.route("**/api/v1/wechat-notify/settings", async (route) => {
    if (route.request().method() !== "POST") return route.fallback();
    await route.fulfill({
      status: 422,
      contentType: "application/json",
      body: JSON.stringify({ code: "H4_REQUEST_INVALID", message: "保存参数失败", severity: "error", details: {}, trace_id: "e2e" }),
    });
  });
  await settingsPage.getByRole("button", { name: "修改" }).click();
  await settingsDialog.getByRole("button", { name: "测试" }).click();
  await expect(settingsDialog).toBeVisible();
  await expect(settingsDialog.getByRole("alert")).toHaveText("保存参数失败");
  expect(testCalledAfterSaveFailure).toBe(false);
  page.off("request", trackBlockedTestRequest);
  await page.unroute("**/api/v1/wechat-notify/settings");
  await settingsDialog.getByRole("button", { name: "取消" }).click();
  await page.screenshot({ path: path.join(artifactsDir, "wechat-settings.png"), fullPage: false });

  await page.getByRole("button", { name: /H4 通知配置/ }).click();
  await expect(page.getByRole("heading", { name: "H4 通知配置" })).toBeVisible();
  const configsPage = page.getByRole("heading", { name: "H4 通知配置" }).locator("..");
  await expect(configsPage.getByText("asn_arrived").first()).toBeVisible();
  await page.screenshot({ path: path.join(artifactsDir, "notify-configs.png"), fullPage: false });

  await page.getByRole("button", { name: /H4 发送记录/ }).click();
  await expect(page.getByRole("heading", { name: "H4 发送记录" })).toBeVisible();
  const recordsPage = page.getByRole("heading", { name: "H4 发送记录" }).locator("..");
  const sentRow = recordsPage.getByRole("row").filter({ hasText: "ASN-H4-E2E", visible: true }).first();
  await expect(sentRow).toBeVisible();
  await expect(sentRow.getByText("asn_arrived")).toBeVisible();
  await sentRow.getByRole("checkbox").check();
  const resendResponsePromise = page.waitForResponse((response) =>
    response.url().includes("/api/v1/wechat-notify/records/")
      && response.url().includes("/resend")
      && response.request().method() === "POST",
  );
  page.once("dialog", (dialog) => void dialog.accept());
  await recordsPage.getByRole("button", { name: "重发" }).click();
  const resendResponse = await resendResponsePromise;
  await expect(resendResponse.json()).resolves.toMatchObject({ status: "failed", retry_count: 1 });
  await expect(recordsPage.getByRole("status")).toHaveText("重发失败：企业微信外部发送能力尚未启用");

  await recordsPage.getByRole("button", { name: "展开" }).click();
  await recordsPage.getByLabel("创建时间开始").fill("2026-07-09");
  await recordsPage.getByLabel("创建时间结束").fill("2026-07-09");
  const dateRequestPromise = page.waitForRequest((request) =>
    request.url().includes("/api/v1/wechat-notify/records?") && request.url().includes("from="),
  );
  await recordsPage.getByRole("button", { name: "查询", exact: true }).click();
  const dateRequest = await dateRequestPromise;
  const expectedDateBoundary = await page.evaluate(() => ({
    from: new Date(2026, 6, 9, 0, 0, 0, 0).toISOString(),
    to: new Date(2026, 6, 9, 23, 59, 59, 999).toISOString(),
  }));
  const dateUrl = new URL(dateRequest.url());
  expect(dateUrl.searchParams.get("from")).toBe(expectedDateBoundary.from);
  expect(dateUrl.searchParams.get("to")).toBe(expectedDateBoundary.to);
  await page.screenshot({ path: path.join(artifactsDir, "notify-records.png"), fullPage: false });
});
